@testset "log timeline: newest first + limit 0 unlimited" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="G", status=:unverified)
    g.attrs["t_created"] = "2020-01-01T00:00:00Z"
    g.attrs["t_updated"] = "2021-06-01T00:00:00Z"
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    rows = M.log_timeline(st; limit=0)
    @test rows[1].ts == "2021-06-01T00:00:00Z"
    @test any(r -> occursin("\tupdated\t", r.line), rows)
end

@testset "log timeline: edge rows" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="x", status=:unverified)
    g.attrs["t_created"] = "2026-01-01T00:00:00Z"
    g.attrs["t_updated"] = g.attrs["t_created"]
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="y", type=:feature, status=:proposed, cynefin=:clear)
    w.attrs["t_created"] = "2026-01-02T00:00:00Z"
    w.attrs["t_updated"] = w.attrs["t_created"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    eg = M.Edge("G-01", :blocks, "W-01")
    eg.t_created = "2026-03-03T12:00:00Z"
    push!(st.edges, eg)
    rows = M.log_timeline(st; limit=10)
    @test any(r -> occursin(r"^\d{4}-\d{2}-\d{2}.*\tedge\t", r.line), rows)
end

@testset "log timeline: id filter + limit cap" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="only", status=:unverified)
    g.attrs["t_created"] = "2020-01-01T00:00:00Z"
    g.attrs["t_updated"] = "2020-03-03T00:00:00Z"
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="other", type=:feature, status=:proposed, cynefin=:clear)
    w.attrs["t_created"] = "2025-05-05T00:00:00Z"
    w.attrs["t_updated"] = w.attrs["t_created"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    r = M.log_timeline(st; idfilt="G-01", limit=1)
    @test length(r) == 1
    @test occursin("G-01", r[1].line)
end

@testset "cli: undo truncates journal and restores state" begin
    tmp = mktempdir()
    try
        jp = joinpath(tmp, ".grove", "journal.log")
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test !isfile(jp)
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=X", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        @test isfile(jp)
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test haskey(st.nodes, "G-01")
        @test M.main(["undo", "--steps=2", "--root=$tmp", "--quiet"]) == 0
        @test !isfile(jp)
        stU = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(stU.nodes, "G-01")
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=Y", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        stR = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test haskey(stR.nodes, "G-01")
        @test stR.nodes["G-01"].title == "Y"
    finally
        rm(tmp; recursive=true)
    end
end

@testset "log timeline: merges journal.log" begin
    st = M.State()
    d = mktempdir()
    jp = joinpath(d, "x.log")
    try
        invd = Dict("op" => "rm_node", "id" => "G-99")
        rec = Dict("v" => 1, "ts" => "2031-01-01T00:00:00Z", "cmd" => "add", "inv" => invd)
        open(jp, "w") do io
            println(io, JSON.json(rec))
        end
        rows = M.log_timeline(st; journal_path = jp, limit = 20)
        @test any(r -> occursin("\tjournal\tadd\trm_node id=G-99", r.line), rows)
        r2 = M.log_timeline(st; idfilt = "G-99", journal_path = jp, limit = 20)
        @test length(r2) >= 1
    finally
        rm(d; recursive = true)
    end
end

@testset "log: journal_file_mentions_id" begin
    d = mktempdir()
    jp = joinpath(d, "j.log")
    try
        rec = Dict(
            "v" => 1,
            "ts" => "2032-01-01T00:00:00Z",
            "cmd" => "add",
            "inv" => Dict("op" => "rm_node", "id" => "G-77"),
        )
        open(jp, "w") do io
            println(io, JSON.json(rec))
        end
        @test M.journal_file_mentions_id(jp, "G-77")
        @test !M.journal_file_mentions_id(jp, "G-99")
    finally
        rm(d; recursive = true)
    end
end

@testset "cli: log" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=T", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        cap = joinpath(tmp, "_log.txt")
        rc = open(cap, "w") do fh
            redirect_stdout(fh) do
                M.main(["log", "--root=$tmp", "--limit=500"])
            end
        end
        @test rc == 0
        blob = read(cap, String)
        @test occursin('\t', blob)
        @test occursin("node", blob)
        @test occursin("\tjournal\t", blob)
        @test M.main(["log", "X-zz", "--root=$tmp"]) == 5
    finally
        rm(tmp; recursive=true)
    end
end

@testset "cli: session tokens on progress" begin
    tmp = mktempdir()
    try
        alice = "--session=alice"
        bob = "--session=bob"
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=T",
                      "--root=$tmp", "--quiet"]) == 0
        for (fn, ln) in (("ac", "a"), ("hypothesis", "h"), ("evidence_strategy", "s"))
            @test M.main(["field", "W-01", fn, "add", ln, "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=ready", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=progress", alice, "--root=$tmp", "--quiet"]) == 0
        lk = joinpath(tmp, ".grove", "state.lock")
        w0 = M.read_lock(lk; verify=false).nodes["W-01"]
        @test M.progress_has_session_record(w0)
        @test strip(w0.attrs["session"]) == "alice"
        @test M.main(["fitness", "W-01", "G-01", "2", "--session=other", "--root=$tmp"]) == M.EXIT_GUARD
        @test M.main(["resume", "W-01", bob, "--root=$tmp", "--quiet"]) == 0
        @test strip(M.read_lock(lk).nodes["W-01"].attrs["session"]) == "bob"
        @test M.main(["fitness", "W-01", "G-01", "3", bob, "--root=$tmp", "--quiet"]) == 0
        @test M.main(["handoff", "W-01", "--to=carol", bob, "--root=$tmp", "--quiet"]) == 0
        @test strip(M.read_lock(lk).nodes["W-01"].attrs["session"]) == "carol"
        @test M.main(["handoff", "W-01", "--to=bob", bob, "--root=$tmp"]) == M.EXIT_GUARD
        @test M.main(["revert", "W-01", "--session=stranger", "--root=$tmp"]) == M.EXIT_GUARD
        stp = M.read_lock(lk; verify=false)
        stp.nodes["W-01"].attrs["session_at"] = "2020-01-01T00:00:00Z"
        M.write_lock(lk, stp)
        @test M.main(["revert", "W-01", "--session=stranger", "--root=$tmp", "--quiet"]) == 0
        wend = M.read_lock(lk).nodes["W-01"]
        @test wend.status === :ready
        @test !M.progress_has_session_record(wend)
    finally
        rm(tmp; recursive=true)
    end
end

@testset "session lock: read vs mutate classification" begin
    for c in keys(M.COMMANDS)
        r = c in M.SESSION_READ_COMMANDS
        m = c in M.SESSION_MUTATE_COMMANDS
        @test xor(r, m)
    end
    @test isempty(intersect(M.SESSION_READ_COMMANDS, M.SESSION_MUTATE_COMMANDS))
end
