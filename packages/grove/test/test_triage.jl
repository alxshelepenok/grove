function triage_run_cli(args)
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

function triage_fixture(tmp)
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=G1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=G2", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=NoSurface", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=ZeroCov", "--surface=src/z.jl", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=FullCovOpenQ", "--surface=src/a.jl", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-02",
                  "--title=Fragile", "--surface=src/a.jl", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-02",
                  "--title=Clean", "--surface=src/a.jl", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "q", "--cynefin=complicated", "--targets=W-02",
                  "--title=QZero", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "q", "--cynefin=complicated", "--targets=W-03",
                  "--title=QFull", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "y", "--title=Discovery", "--tags=auth", "--surface=src/a.jl",
                  "--from=W-03", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
    for fn in ("ac", "hypothesis", "evidence_strategy")
        for wid in ("W-03", "W-04", "W-05")
            @test M.main(["field", wid, fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
    end
    @test M.main(["fitness", "W-03", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["fitness", "W-04", "G-02", "+1", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["fitness", "W-05", "G-02", "+1", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear",
                  "--title=PathA", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear",
                  "--title=PathB", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["link", "G-02", "blocks", "W-06", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["link", "G-02", "blocks", "W-07", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["link", "W-06", "blocks", "W-04", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["link", "W-06", "blocks", "W-05", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["link", "W-07", "blocks", "W-05", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-06", "status=rejected", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-07", "status=rejected", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear",
                  "--title=DoneW", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["evidence", "W-08", "shipped", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-08", "status=done", "--root=$tmp", "--quiet"]) == 0
end

@testset "triage: ranks open work by coverage, uncertainty, id" begin
    tmp = mktempdir()
    try
        triage_fixture(tmp)
        rc, txt = triage_run_cli(["triage", "--root=$tmp"])
        @test rc == 0
        lines = split(strip(txt), '\n')
        @test lines[1] == "W\tcov\tχ\tfragile\tsuggestion"
        @test length(lines) == 6
        @test lines[2] == "W-02\t0.00\t6\tyes\tspike to create coverage"
        @test lines[3] == "W-01\t0.00\t4\tyes\tdeclare surface"
        @test lines[4] == "W-03\t1.00\t2\tyes\tresolve open Q/B and DoR gaps"
        @test lines[5] == "W-04\t1.00\t0\tyes\tadd a redundant path (blocks)"
        @test lines[6] == "W-05\t1.00\t0\tno\tready to deliver"
        @test !occursin("W-06", txt)
        @test !occursin("W-07", txt)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "triage: deterministic across repeated runs" begin
    tmp = mktempdir()
    try
        triage_fixture(tmp)
        rc1, txt1 = triage_run_cli(["triage", "--root=$tmp"])
        rc2, txt2 = triage_run_cli(["triage", "--root=$tmp"])
        @test rc1 == 0
        @test rc2 == 0
        @test txt1 == txt2
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "triage: json shape" begin
    tmp = mktempdir()
    try
        triage_fixture(tmp)
        rc, txt = triage_run_cli(["triage", "--json", "--root=$tmp"])
        @test rc == 0
        d = JSON.parse(txt)
        @test d["command"] == "triage"
        rows = d["rows"]
        @test length(rows) == 5
        @test [r["w"] for r in rows] == ["W-02", "W-01", "W-03", "W-04", "W-05"]
        for r in rows
            @test sort!(collect(keys(r))) ==
                  ["coverage", "declared", "fragile", "suggestion", "title", "uncertainty", "w"]
        end
        @test rows[1]["coverage"] == 0.0
        @test rows[1]["declared"] == true
        @test rows[1]["uncertainty"] == 6
        @test rows[1]["fragile"] == true
        @test rows[1]["suggestion"] == "spike to create coverage"
        @test rows[1]["title"] == "ZeroCov"
        @test rows[2]["declared"] == false
        @test rows[2]["suggestion"] == "declare surface"
        @test rows[3]["suggestion"] == "resolve open Q/B and DoR gaps"
        @test rows[4]["suggestion"] == "add a redundant path (blocks)"
        @test rows[5]["fragile"] == false
        @test rows[5]["suggestion"] == "ready to deliver"
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "triage: read-only, lock and journal untouched" begin
    tmp = mktempdir()
    try
        triage_fixture(tmp)
        lock = joinpath(tmp, ".grove", "state.lock")
        jp = joinpath(tmp, ".grove", "journal.log")
        lock_before = read(lock)
        journal_before = read(jp)
        rc, _ = triage_run_cli(["triage", "--root=$tmp"])
        @test rc == 0
        rcj, _ = triage_run_cli(["triage", "--json", "--root=$tmp"])
        @test rcj == 0
        @test read(lock) == lock_before
        @test read(jp) == journal_before
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "triage: empty project prints no open work" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        rc, txt = triage_run_cli(["triage", "--root=$tmp"])
        @test rc == 0
        @test strip(txt) == "triage: no open work"
        rcj, txtj = triage_run_cli(["triage", "--json", "--root=$tmp"])
        @test rcj == 0
        d = JSON.parse(txtj)
        @test d["command"] == "triage"
        @test d["rows"] == []
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "triage: classified read, never mutate" begin
    @test "triage" in M.SESSION_READ_COMMANDS
    @test !("triage" in M.SESSION_MUTATE_COMMANDS)
    @test xor("triage" in M.SESSION_READ_COMMANDS, "triage" in M.SESSION_MUTATE_COMMANDS)
end
