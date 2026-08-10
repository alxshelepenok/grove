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
