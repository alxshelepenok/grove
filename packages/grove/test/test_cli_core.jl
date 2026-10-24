@testset "cli: init writes lock index then add goal passes check" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test isfile(joinpath(tmp, ".grove", "state.lock"))
    @test isfile(joinpath(tmp, ".grove", "index.md"))
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=Migrate", "--fitness-kind=count", "--fitness-target=5", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=Add login", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["check", "--root=$tmp", "--quiet"]) == 0
    @test !isdir(joinpath(tmp, ".grove", "locks", "exclusive"))
    rm(tmp; recursive=true)
end

@testset "cli: set goal fitness recomputes legacy status immediately" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=T", "--fitness-kind=count", "--fitness-target=10", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+5", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=ready", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].status == :partial
        @test M.main(["set", "G-01", "fitness_target=5", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st2.nodes["G-01"].status == :verified
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: json mode emits structured payloads for check dor status deps" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=X", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        function run_json_cmd(args)
            out_path, out_io = mktemp()
            close(out_io)
            rc = Ref(-1)
            open(out_path, "w") do f
                redirect_stdout(f) do
                    rc[] = M.main(args)
                end
            end
            txt = read(out_path, String)
            rm(out_path, force=true)
            rc[], JSON.parse(txt)
        end
        r, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "check"
        @test d["ok"] == true
        r, d = run_json_cmd(["dor", "W-01", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "dor"
        @test d["work"] == "W-01"
        @test d["dor"] isa Bool
        @test length(d["conjuncts"]) >= 3
        r, d = run_json_cmd(["status", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "status"
        @test haskey(d, "progress") && haskey(d, "invariants")
        r, d = run_json_cmd(["deps", "W-01", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "deps"
        @test d["predecessors"] isa AbstractVector
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "protocol: chaotic question surfaces alignment trigger" begin
    st = M.State()
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:open, cynefin=:chaotic)
    t = M.alignment_triggers(st)
    @test any(x -> occursin("chaotic", x), t)
end

@testset "cli: status exits zero on fresh repo" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["status", "--root=$tmp", "--quiet"]) == 0
    rm(tmp; recursive=true)
end

@testset "cli: add prints the assigned ID in text and json modes" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        function capture(args)
            out_path, out_io = mktemp()
            close(out_io)
            rc = Ref(-1)
            open(out_path, "w") do f
                redirect_stdout(f) do
                    rc[] = M.main(args)
                end
            end
            txt = read(out_path, String)
            rm(out_path, force=true)
            rc[], txt
        end
        r, txt = capture(["add", "a", "--title=Area", "--root=$tmp", "--quiet"])
        @test r == 0
        @test txt == "A-01\n"
        r, txt = capture(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet", "--json"])
        @test r == 0
        d = JSON.parse(txt)
        @test d["command"] == "add"
        @test d["kind"] == "g"
        @test d["id"] == "G-01"
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: accepted decision rejects field and set title mutations" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=Original decision", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "D-01", "decision", "add", "Original body.", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        for fn in string.(M.FIELD_ORDER[:d])
            @test M.main(["field", "D-01", fn, "add", "Replacement", "--root=$tmp", "--quiet"]) == 4
            @test M.main(["field", "D-01", fn, "clear", "--root=$tmp", "--quiet"]) == 4
            @test M.main(["field", "D-01", fn, "rm", "1", "--root=$tmp", "--quiet"]) == 4
        end
        for kv in ("title=Replacement title", "cynefin=complicated", "type=feature", "status=proposed")
            @test M.main(["set", "D-01", kv, "--root=$tmp", "--quiet"]) == 4
        end
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["D-01"].fields[:decision] == ["Original body."]
        @test st.nodes["D-01"].title == "Original decision"
        @test st.nodes["D-01"].status == :accepted
        @test M.main(["add", "d", "--title=Replacement decision", "--supersedes=D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "D-02", "decision", "add", "New body.", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-02", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st2.nodes["D-02"].status == :accepted
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st3 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st3.nodes["D-02"].status == :proposed
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: unknown field and unsupported key diagnostics precede the accepted-decision guard" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=T", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "D-01", "decision", "add", "B", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "D-01", "nosuch", "add", "x", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["set", "D-01", "nosuchkey=V", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["field", "D-01", "decision", "add", "x", "--root=$tmp", "--quiet"]) == 4
        @test M.main(["set", "D-01", "title=Replacement", "--root=$tmp", "--quiet"]) == 4
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["D-01"].fields[:decision] == ["B"]
        @test st.nodes["D-01"].title == "T"
        @test st.nodes["D-01"].status == :accepted
    finally
        rm(tmp; recursive=true, force=true)
    end
end

function capture_stderr_core(args)
    out_path, out_io = mktemp()
    close(out_io)
    rc = Ref(-1)
    open(out_path, "w") do f
        redirect_stderr(f) do
            rc[] = M.main(args)
        end
    end
    txt = read(out_path, String)
    rm(out_path, force=true)
    rc[], txt
end

@testset "cli: status=done guards blocks clearance and spike outputs (issue 71)" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1",
                      "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "q", "--title=Unresolved prerequisite", "--cynefin=complicated",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=Dependent work", "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "Q-01", "blocks", "W-01", "--root=$tmp", "--quiet"]) == 0
        # control: progress entry rejects with I5 (issue 71 setup)
        rc, err = capture_stderr_core(["set", "W-01", "status=progress", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("I5: predecessors not cleared", err)
        # the fix: done rejects with the same I5 text before evidence validation
        rc, err = capture_stderr_core(["set", "W-01", "status=done", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("I5: predecessors not cleared", err)
        # completion succeeds after legitimate predecessor clearance
        @test M.main(["set", "Q-01", "status=answered", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].status == :done
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: done spike without produces rejects with I2 before evidence" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1",
                      "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=spike", "--cynefin=complex", "--goals=G-01",
                      "--title=Probe", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        # I2 precedes I3: no evidence staged yet, message must still be I2
        rc, err = capture_stderr_core(["set", "W-01", "status=done", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("I2: W-01 is a done spike but `produces` is empty", err)
        # producing a decision clears the gate
        @test M.main(["add", "d", "--title=Finding", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].status == :done
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: manual goal verification restricted to manual-fitness goals" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=Counted", "--fitness-kind=count", "--fitness-target=1",
                      "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=Judged", "--fitness-kind=manual",
                      "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        # structured kind: derived status, human set rejected at any fitness level
        rc, err = capture_stderr_core(["set", "G-01", "status=verified", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("derived from fitness deltas", err)
        # manual kind: human verdict passes, and undo reverts it
        @test M.main(["set", "G-02", "status=verified", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-02"].status == :verified
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-02"].status == :unverified
        # declined stays a settable human verdict on a structured goal
        @test M.main(["set", "G-01", "status=declined", "--root=$tmp", "--quiet"]) == 0
        # add-time bypass: --status=verified rejected for structured, allowed for manual
        rc, err = capture_stderr_core(["add", "g", "--title=Sneaky", "--fitness-kind=count",
                                       "--fitness-target=3", "--area=A-01",
                                       "--status=verified", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("derived from fitness deltas", err)
        @test M.main(["add", "g", "--title=Judge2", "--fitness-kind=manual", "--area=A-01",
                      "--status=verified", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-03"].status == :verified
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: terminal nodes freeze set field fitness evidence (I14)" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=manual", "--area=A-01",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=W", "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        # frozen: every user-fired mutation command on the terminal work item
        for args in (["set", "W-01", "title=X"],
                     ["set", "W-01", "status=archived"],
                     ["field", "W-01", "ac", "add", "y"],
                     ["evidence", "W-01", "more"],
                     ["fitness", "W-01", "G-01", "+1"])
            rc, err = capture_stderr_core([args..., "--root=$tmp"])
            @test rc == M.EXIT_GUARD
            @test occursin("W-01 is terminal (done); create a successor node or use grove undo", err)
        end
        # undo remains a sanctioned lane: reverts the done transition itself
        # (this item went proposed -> done directly, so the inverse restores proposed)
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].status == :proposed
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test isempty(get(st.nodes["W-01"].fields, :evidence, String[]))
        # rejected and superseded decisions freeze like every other terminal kind
        @test M.main(["add", "d", "--title=Rejected", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=rejected", "--root=$tmp", "--quiet"]) == 0
        rc, err = capture_stderr_core(["set", "D-01", "title=X", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("D-01 is terminal (rejected)", err)
        # accepted decision keeps its supersede-lane message and its one legal transition
        @test M.main(["add", "d", "--title=Accepted", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-02", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        rc, err = capture_stderr_core(["set", "D-02", "title=X", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("decision D-02 is accepted; create a new D with --supersedes", err)
        @test M.main(["add", "d", "--title=Newer", "--supersedes=D-02", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-03", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-02", "status=superseded", "--root=$tmp", "--quiet"]) == 0
        # terminal question freezes too
        @test M.main(["add", "q", "--title=Q", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Q-01", "status=answered", "--root=$tmp", "--quiet"]) == 0
        rc, err = capture_stderr_core(["field", "Q-01", "outcome", "add", "late",
                                       "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("Q-01 is terminal (answered)", err)
    finally
        rm(tmp; recursive=true, force=true)
    end
end
