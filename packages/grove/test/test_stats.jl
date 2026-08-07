function stats_capture_err(args)
    err_path, err_io = mktemp()
    close(err_io)
    rc = -1
    open(err_path, "w") do f
        redirect_stderr(f) do
            rc = M.main(args)
        end
    end
    txt = read(err_path, String)
    rm(err_path; force=true)
    (rc, txt)
end

function stats_capture_out(args)
    out_path, out_io = mktemp()
    close(out_io)
    rc = -1
    open(out_path, "w") do f
        redirect_stdout(f) do
            rc = M.main(args)
        end
    end
    txt = read(out_path, String)
    rm(out_path, force=true)
    (rc, txt)
end

function stats_run_json(args)
    rc, txt = stats_capture_out(args)
    (rc, JSON.parse(txt))
end

stats_rec(ts, cmd, inv) = Dict{String,Any}("v" => 1, "ts" => ts, "cmd" => cmd, "inv" => inv)

@testset "stats: empty journal on bare init yields zeroed metrics" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        rc, d = stats_run_json(["stats", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["command"] == "stats"
        @test d["records"] == 0
        @test d["mutations"] == 0
        @test isempty(d["cycle_time"]["by_cynefin"])
        @test isempty(d["cycle_time"]["durations_seconds"])
        @test d["dor"]["reject_events"] == 0
        @test d["dor"]["progress_entries"] == 0
        @test d["dor"]["first_pass_rate"] === nothing
        @test d["bets"]["ratio"] === nothing
        @test d["discovery"]["gate_runs"] == 0
        @test d["undo"]["undo_events"] == 0
        @test d["undo"]["undos_per_100_mutations"] === nothing
        @test d["surprise"]["total"] == 0
        @test d["surprise"]["per_done"] === nothing
        @test length(d["cv_series"]) == 1
        @test d["replay_failures"] == 0
        rc2, txt = stats_capture_out(["stats", "--root=$tmp"])
        @test rc2 == 0
        @test occursin("cycle time", txt)
        @test occursin("cv series", txt)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: metrics agree with hand computation on crafted records" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="b1", status=:validated, cynefin=:clear)
    st.nodes["B-02"] = M.Node(:b, "B-02"; title="b2", status=:invalidated_blocking, cynefin=:clear)
    st.nodes["Y-01"] = M.Node(:y, "Y-01"; title="x", status=:stale)
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-01")),
        stats_rec("2026-01-01T00:30:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-01")),
        stats_rec("2026-01-01T00:31:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-02")),
        stats_rec("2026-01-01T00:35:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "Y-01")),
        stats_rec("2026-01-01T01:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "proposed", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T02:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-01", "old_status" => "testing")),
        stats_rec("2026-01-01T03:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-01", "missing" => ["goals(w) ≠ ∅"])),
        stats_rec("2026-01-01T04:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-02", "old_status" => "testing")),
        stats_rec("2026-01-01T05:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "ready", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T06:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "Y-01", "old_status" => "active")),
        stats_rec("2026-01-01T07:00:00Z", "gate", Dict{String,Any}(
            "op" => "gate", "tw" => 1, "dones" => 0, "empty" => false,
            "overflows" => ["W-01"], "invalidated" => ["B-02"])),
        stats_rec("2026-01-01T09:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "progress", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T10:00:00Z", "undo",
            Dict{String,Any}("op" => "undo", "steps" => 1)),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    @test s["records"] == 13
    @test s["mutations"] == 10
    ct = s["cycle_time"]["by_cynefin"]["clear"]
    @test ct["n"] == 1
    @test ct["mean_hours"] == 8.0
    @test ct["median_hours"] == 8.0
    @test ct["max_hours"] == 8.0
    @test s["cycle_time"]["durations_seconds"] == [28800]
    @test s["dor"]["reject_events"] == 1
    @test s["dor"]["reject_per_node"] == Dict("W-01" => 1)
    @test s["dor"]["progress_entries"] == 1
    @test s["dor"]["first_pass"] == 0
    @test s["dor"]["first_pass_rate"] == 0.0
    @test s["bets"]["validated"] == 1
    @test s["bets"]["invalidated_acceptable"] == 0
    @test s["bets"]["invalidated_blocking"] == 1
    @test s["bets"]["ratio"] == 1.0
    @test s["discovery"]["stale_entries"] == 1
    @test s["discovery"]["revalidations"] == 0
    @test s["discovery"]["gate_runs"] == 1
    @test s["discovery"]["gate_overflow_events"] == 1
    @test s["discovery"]["gate_invalidated_events"] == 1
    @test s["undo"]["undo_events"] == 1
    @test s["undo"]["undone_steps"] == 1
    @test s["undo"]["undos_per_100_mutations"] == 10.0
    @test s["surprise"]["total"] == 2
    @test s["surprise"]["done_w"] == 1
    @test s["surprise"]["per_done"] == 2.0
    @test s["replay_failures"] == 0
    @test length(s["cv_series"]) == 11
    @test s["cv_series"][1] == Dict("ts" => "2026-01-01T00:00:00Z", "c" => 0, "v" => 0)
    @test s["cv_series"][end] == Dict("ts" => "2026-01-02T00:00:00Z", "c" => 1, "v" => 0)
end

@testset "stats: DoR guard journals dor_reject and undo skips it" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        rc, etxt = stats_capture_err(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"])
        @test rc == M.EXIT_GUARD
        @test occursin("DoR", etxt)
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        last = recs[end]
        @test last["cmd"] == "set"
        @test last["inv"]["op"] == "dor_reject"
        @test last["inv"]["id"] == "W-01"
        @test last["inv"]["missing"] isa Vector
        @test !isempty(last["inv"]["missing"])
        @test M.main(["undo", "--steps=1", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st.nodes, "W-01")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: undo journals a non-mutation undo record" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["undo", "--steps=1", "--root=$tmp", "--quiet"]) == 0
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        @test recs[end]["cmd"] == "undo"
        @test recs[end]["inv"]["op"] == "undo"
        @test recs[end]["inv"]["steps"] == 1
        @test M.main(["undo", "--steps=1", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st.nodes, "A-01")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: gate record carries overflows and invalidated lists" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["gate", "--root=$tmp", "--quiet"]) == 0
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        g = recs[end]
        @test g["cmd"] == "gate"
        @test g["inv"]["op"] == "gate"
        @test haskey(g["inv"], "overflows")
        @test haskey(g["inv"], "invalidated")
        @test haskey(g["inv"], "overflow_counts")
        @test isempty(g["inv"]["overflow_counts"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: gates array exposes per-gate rows" begin
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "gate", Dict{String,Any}(
            "op" => "gate", "tw" => 1, "dones" => 2, "empty" => false,
            "overflows" => ["W-01"], "overflow_counts" => Dict{String,Any}("W-01" => 3),
            "invalidated" => ["B-01", "B-02"])),
        stats_rec("2026-01-02T00:00:00Z", "gate", Dict{String,Any}(
            "op" => "gate", "tw" => 1, "dones" => 0, "empty" => true,
            "overflows" => [], "invalidated" => [])),
    ]
    s = M.compute_stats(M.State(), recs; now_ts="2026-01-03T00:00:00Z")
    @test s["gates"] == Dict{String,Any}[
        Dict{String,Any}("ts" => "2026-01-01T00:00:00Z", "tw" => 1, "dones" => 2,
            "empty" => false, "overflow_events" => 1, "overflow_paths" => 3,
            "invalidated_events" => 2),
        Dict{String,Any}("ts" => "2026-01-02T00:00:00Z", "tw" => 1, "dones" => 0,
            "empty" => true, "overflow_events" => 0, "overflow_paths" => nothing,
            "invalidated_events" => 0),
    ]
    @test s["discovery"]["gate_empty"] == 1
end

@testset "stats: end to end on a small project with a done W" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness=1/1", "--area=A-01",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=W", "--root=$tmp", "--quiet"]) == 0
        for fn in ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=ready", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        rc, d = stats_run_json(["stats", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["command"] == "stats"
        @test d["records"] == 11
        @test d["mutations"] == 11
        ct = d["cycle_time"]["by_cynefin"]["clear"]
        @test ct["n"] == 1
        @test ct["mean_hours"] >= 0
        @test d["cycle_time"]["durations_seconds"][1] >= 0
        @test d["dor"]["progress_entries"] == 1
        @test d["dor"]["first_pass"] == 1
        @test d["dor"]["first_pass_rate"] == 1.0
        @test d["undo"]["undo_events"] == 0
        @test d["surprise"]["done_w"] == 1
        @test d["replay_failures"] == 0
        @test length(d["cv_series"]) == 12
        rc2, txt = stats_capture_out(["stats", "--root=$tmp"])
        @test rc2 == 0
        @test occursin("clear", txt)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: journal records carry the effective session token" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--session=tok-x", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Other", "--root=$tmp", "--quiet"]) == 0
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        @test recs[1]["session"] == "tok-x"
        @test recs[2]["session"] == M.effective_session_token(tmp, Dict{String,String}())
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: audit groups commands per session with unknown bucket" begin
    recs = Dict{String,Any}[
        merge(stats_rec("2026-01-01T00:00:00Z", "add",
                Dict{String,Any}("op" => "rm_node", "id" => "W-01")),
            Dict{String,Any}("session" => "tok-a")),
        merge(stats_rec("2026-01-01T01:00:00Z", "set",
                Dict{String,Any}("op" => "set_title", "id" => "W-01", "old" => "x")),
            Dict{String,Any}("session" => "tok-a")),
        stats_rec("2026-01-01T02:00:00Z", "gate",
            Dict{String,Any}("op" => "gate", "tw" => 1, "dones" => 0, "empty" => true,
                "overflows" => [], "invalidated" => [])),
    ]
    s = M.compute_stats(M.State(), recs; now_ts="2026-01-02T00:00:00Z")
    sess = s["audit"]["sessions"]
    @test sess["count"] == 2
    @test sess["per_session"] == Dict{String,Any}[
        Dict{String,Any}("session" => "tok-a", "commands" => 2),
        Dict{String,Any}("session" => "unknown", "commands" => 1),
    ]
    @test sess["mean"] == 1.5
    @test sess["median"] == 1.5
    @test sess["max"] == 2
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["audit"]["sessions"]["count"] == 0
    @test e["audit"]["sessions"]["mean"] === nothing
    @test e["audit"]["sessions"]["max"] === nothing
end

@testset "stats: archive record is audit-only and not undoable" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "G-01", "status=verified", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$tmp", "--quiet"]) == 0
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        last = recs[end]
        @test last["cmd"] == "archive"
        @test last["inv"]["op"] == "archive"
        @test last["inv"]["id"] == "G-01"
        @test last["inv"]["ids"] == ["G-01"]
        @test haskey(last, "session")
        @test haskey(recs[end-1], "session")
        @test !M.journal_record_mutation(last)
        @test M.journal_apply_inverse!(M.State(), last["inv"]) isa String
        @test M.main(["undo", "--steps=1", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].archived
        @test st.nodes["G-01"].status == :unverified
        _, recs2 = M.journal_read_nonempty_pairs(jp)
        @test any(r -> get(r, "cmd", "") == "archive", recs2)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "stats: checkpoint latency both series plus empty case" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["Y-01"] = M.Node(:y, "Y-01"; title="y", status=:active)
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-01")),
        stats_rec("2026-01-01T00:30:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "Y-01")),
        stats_rec("2026-01-01T01:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-01", "missing" => ["goals(w) ≠ ∅"])),
        stats_rec("2026-01-01T03:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "ready", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T04:30:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "Y-01", "old_status" => "proposed")),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    dor = s["audit"]["checkpoint_latency"]["dor"]
    @test dor["n"] == 1
    @test dor["mean_hours"] == 2.0
    @test dor["median_hours"] == 2.0
    @test dor["max_hours"] == 2.0
    disc = s["audit"]["checkpoint_latency"]["discovery"]
    @test disc["n"] == 1
    @test disc["mean_hours"] == 4.0
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["audit"]["checkpoint_latency"]["dor"] ==
          Dict{String,Any}("n" => 0, "mean_hours" => nothing,
                           "median_hours" => nothing, "max_hours" => nothing)
    @test e["audit"]["checkpoint_latency"]["discovery"]["n"] == 0
    @test e["audit"]["checkpoint_latency"]["discovery"]["mean_hours"] === nothing
    @test e["surprise_series"] == []
end

@testset "stats: post-approval invalidation rate" begin
    st = M.State()
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="b1", status=:invalidated_blocking, cynefin=:clear)
    st.nodes["B-02"] = M.Node(:b, "B-02"; title="b2", status=:validated, cynefin=:clear)
    st.nodes["B-03"] = M.Node(:b, "B-03"; title="b3", status=:testing, cynefin=:clear)
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-01")),
        stats_rec("2026-01-01T00:10:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-02")),
        stats_rec("2026-01-01T00:20:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-03")),
        stats_rec("2026-01-01T01:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-01", "old_status" => "testing")),
        stats_rec("2026-01-01T01:30:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-02", "old_status" => "testing")),
        stats_rec("2026-01-01T02:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-01", "old_status" => "validated")),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    pai = s["audit"]["post_approval_invalidation"]
    @test pai["invalidated"] == 1
    @test pai["ever_validated"] == 2
    @test pai["rate"] == 0.5
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["audit"]["post_approval_invalidation"]["rate"] === nothing
end

@testset "stats: rework covered/uncovered split with reject counts" begin
    st = M.State()
    w1 = M.Node(:w, "W-01"; title="a", type=:feature, status=:ready, cynefin=:clear)
    w1.fields[:surface] = ["src/a.jl"]
    w2 = M.Node(:w, "W-02"; title="b", type=:feature, status=:ready, cynefin=:clear)
    w2.fields[:surface] = ["src/b.jl"]
    w3 = M.Node(:w, "W-03"; title="c", type=:feature, status=:ready, cynefin=:clear)
    w4 = M.Node(:w, "W-04"; title="d", type=:feature, status=:done, cynefin=:clear)
    w4.archived = true
    w4.fields[:surface] = ["src/a.jl"]
    y1 = M.Node(:y, "Y-01"; title="y", status=:active)
    y1.fields[:surface] = ["src/a.jl"]
    y2 = M.Node(:y, "Y-02"; title="y2", status=:stale)
    y2.fields[:surface] = ["src/b.jl"]
    for n in (w1, w2, w3, w4, y1, y2)
        st.nodes[n.id] = n
    end
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-01", "missing" => ["x"])),
        stats_rec("2026-01-01T01:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-01", "missing" => ["x"])),
        stats_rec("2026-01-01T02:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-03", "missing" => ["x"])),
        stats_rec("2026-01-01T03:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-04", "missing" => ["x"])),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    cov = s["rework"]["covered"]
    @test cov["w"] == 2
    @test cov["rejects"] == 3
    @test cov["mean_rejects"] == 1.5
    @test cov["per_w"] == Dict{String,Any}[
        Dict{String,Any}("id" => "W-01", "rejects" => 2),
        Dict{String,Any}("id" => "W-04", "rejects" => 1),
    ]
    unc = s["rework"]["uncovered"]
    @test unc["w"] == 2
    @test unc["rejects"] == 1
    @test unc["mean_rejects"] == 0.5
    @test unc["per_w"] == Dict{String,Any}[
        Dict{String,Any}("id" => "W-02", "rejects" => 0),
        Dict{String,Any}("id" => "W-03", "rejects" => 1),
    ]
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["rework"]["covered"]["w"] == 0
    @test e["rework"]["covered"]["mean_rejects"] === nothing
    @test e["rework"]["uncovered"]["per_w"] == []
end

@testset "stats: distill yield real/null/none per archived goal" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    g1.archived = true
    g2 = M.Node(:g, "G-02"; title="b", status=:verified)
    g2.archived = true
    g3 = M.Node(:g, "G-03"; title="c", status=:verified)
    g3.archived = true
    g4 = M.Node(:g, "G-04"; title="d", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
    w1.archived = true
    w1.fields[:goals] = ["G-01"]
    d1 = M.Node(:d, "D-01"; title="d", status=:accepted)
    d1.archived = true
    y1 = M.Node(:y, "Y-01"; title="y", status=:active)
    for n in (g1, g2, g3, g4, w1, d1, y1)
        st.nodes[n.id] = n
    end
    push!(st.edges, M.Edge("W-01", :implements, "D-01"))
    push!(st.edges, M.Edge("Y-01", :distills, "D-01"))
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "distill",
            Dict{String,Any}("op" => "distill", "goal" => "G-02", "empty" => true)),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    dy = s["distill_yield"]
    @test dy["goals_with_real"] == 1
    @test dy["goals_null_attested"] == 1
    @test dy["goals_without"] == 1
    @test dy["goals"] == Dict{String,Any}[
        Dict{String,Any}("goal" => "G-01", "status" => "real", "discoveries" => ["Y-01"]),
        Dict{String,Any}("goal" => "G-02", "status" => "null", "discoveries" => String[]),
        Dict{String,Any}("goal" => "G-03", "status" => "none", "discoveries" => String[]),
    ]
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["distill_yield"]["goals"] == []
end

@testset "stats: DoR first-pass split three categories" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="a", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="b", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["W-03"] = M.Node(:w, "W-03"; title="c", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:answered, cynefin=:clear)
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-01")),
        stats_rec("2026-01-01T00:01:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-02")),
        stats_rec("2026-01-01T00:02:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-03")),
        stats_rec("2026-01-01T00:03:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "Q-01")),
        stats_rec("2026-01-01T01:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "ready", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T02:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-02", "missing" => ["x"])),
        stats_rec("2026-01-01T02:30:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "Q-01", "old_status" => "open")),
        stats_rec("2026-01-01T03:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-02",
            "old_w_status" => "ready", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T04:00:00Z", "set",
            Dict{String,Any}("op" => "dor_reject", "id" => "W-03", "missing" => ["x"])),
        stats_rec("2026-01-01T04:30:00Z", "set",
            Dict{String,Any}("op" => "set_title", "id" => "W-01", "old" => "a")),
        stats_rec("2026-01-01T05:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-03",
            "old_w_status" => "ready", "goal_statuses" => Dict{String,Any}())),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    fps = s["dor"]["first_pass_split"]
    @test fps["no_reject"] == 1
    @test fps["reject_discovery"] == 1
    @test fps["reject_plain"] == 1
    @test fps["discovery_rate"] == 0.5
    e = M.compute_stats(M.State(), Dict{String,Any}[]; now_ts="2026-01-02T00:00:00Z")
    @test e["dor"]["first_pass_split"]["discovery_rate"] === nothing
end

@testset "stats: surprise series delta and c assignment" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="a", type=:feature, status=:done, cynefin=:clear)
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="b", type=:feature, status=:done, cynefin=:clear)
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:answered, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="x", status=:validated, cynefin=:clear)
    recs = Dict{String,Any}[
        stats_rec("2026-01-01T00:00:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-01")),
        stats_rec("2026-01-01T00:05:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "W-02")),
        stats_rec("2026-01-01T00:10:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "Q-01")),
        stats_rec("2026-01-01T00:15:00Z", "add",
            Dict{String,Any}("op" => "rm_node", "id" => "B-01")),
        stats_rec("2026-01-01T01:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "Q-01", "old_status" => "open")),
        stats_rec("2026-01-01T02:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-01", "old_status" => "testing")),
        stats_rec("2026-01-01T03:00:00Z", "set",
            Dict{String,Any}("op" => "set_status_plain", "id" => "B-01",
                "old_status" => "invalidated_blocking")),
        stats_rec("2026-01-01T04:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-01",
            "old_w_status" => "progress", "goal_statuses" => Dict{String,Any}())),
        stats_rec("2026-01-01T05:00:00Z", "gate", Dict{String,Any}(
            "op" => "gate", "tw" => 1, "dones" => 1, "empty" => false,
            "overflows" => ["W-01", "W-02"], "invalidated" => [])),
        stats_rec("2026-01-01T06:00:00Z", "set", Dict{String,Any}(
            "op" => "set_w_status_with_goals", "id" => "W-02",
            "old_w_status" => "progress", "goal_statuses" => Dict{String,Any}())),
    ]
    s = M.compute_stats(st, recs; now_ts="2026-01-02T00:00:00Z")
    @test s["surprise_series"] == Dict{String,Any}[
        Dict{String,Any}("id" => "W-01", "ts" => "2026-01-01T04:00:00Z", "delta" => 1, "c" => 2),
        Dict{String,Any}("id" => "W-02", "ts" => "2026-01-01T06:00:00Z", "delta" => 2, "c" => 2),
    ]
end
