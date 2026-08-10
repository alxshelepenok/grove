@testset "cli: undo truncates journal and restores prior lock snapshot" begin
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
        _, recs_u = M.journal_read_nonempty_pairs(jp)
        @test length(recs_u) == 1 && recs_u[1]["inv"]["op"] == "undo"
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

@testset "cli: log prints mixed node journal rows and exits missing filter notfound" begin
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

@testset "cli: session tokens gate mutate resume handoff revert freshness rules" begin
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

@testset "session: each cli command classified read xor mutate exclusively" begin
    for c in keys(M.COMMANDS)
        r = c in M.SESSION_READ_COMMANDS
        m = c in M.SESSION_MUTATE_COMMANDS
        @test xor(r, m)
    end
    @test isempty(intersect(M.SESSION_READ_COMMANDS, M.SESSION_MUTATE_COMMANDS))
end
