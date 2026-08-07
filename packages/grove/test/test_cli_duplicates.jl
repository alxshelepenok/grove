function capture_stderr(args)
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

journal_line_count(tmp) = let jp = joinpath(tmp, ".grove", "journal.log")
    isfile(jp) ? countlines(jp) : 0
end

@testset "cli: field add refuses duplicate reflist entries" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "W-01", "tags", "add", "thin adapter", "--root=$tmp", "--quiet"]) == 0
        lock_before = read(joinpath(tmp, ".grove", "state.lock"), String)
        jlines = journal_line_count(tmp)
        rc, err = capture_stderr(["field", "W-01", "tags", "add", "thin adapter",
                                  "--root=$tmp", "--quiet"])
        @test rc == M.EXIT_GUARD
        @test occursin("W-01", err) && occursin("tags", err) && occursin("thin adapter", err)
        @test read(joinpath(tmp, ".grove", "state.lock"), String) == lock_before
        @test journal_line_count(tmp) == jlines
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].fields[:tags] == ["thin adapter"]
        @test M.main(["field", "W-01", "tags", "add", "other", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st2.nodes["W-01"].fields[:tags] == ["thin adapter", "other"]
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: add refuses duplicate CSV list entries" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        jlines = journal_line_count(tmp)
        rc, err = capture_stderr(["add", "a", "--title=A", "--surface=src/x.jl, src/x.jl",
                                  "--root=$tmp", "--quiet"])
        @test rc == M.EXIT_ERR
        @test occursin("--surface", err) && occursin("src/x.jl", err)
        @test journal_line_count(tmp) == jlines
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test isempty(st.nodes)
        @test M.main(["add", "a", "--title=A", "--surface=src/x.jl, src/y.jl",
                      "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st2.nodes["A-01"].fields[:surface] == ["src/x.jl", "src/y.jl"]
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: add y refuses duplicate tags and w refuses duplicate goals" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        rc, err = capture_stderr(["add", "y", "--title=Y", "--tags=alpha, alpha",
                                  "--surface=p1", "--from=W-01", "--root=$tmp", "--quiet"])
        @test rc == M.EXIT_ERR
        @test occursin("--tags", err) && occursin("alpha", err)
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st.nodes, "Y-01")
        @test M.main(["add", "y", "--title=Y", "--tags=alpha, beta", "--surface=p1",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st2.nodes["Y-01"].fields[:tags] == ["alpha", "beta"]
        rc2, err2 = capture_stderr(["add", "w", "--type=feature", "--cynefin=clear",
                                    "--goals=G-01,G-01", "--title=W2", "--root=$tmp", "--quiet"])
        @test rc2 == M.EXIT_ERR
        @test occursin("--goals", err2) && occursin("G-01", err2)
        st3 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st3.nodes, "W-02")
    finally
        rm(tmp; recursive=true, force=true)
    end
end
