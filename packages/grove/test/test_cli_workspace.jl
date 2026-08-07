@testset "cli: reject fitness_current field when goal kind is structured count" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=T", "--fitness-kind=count", "--fitness-target=3", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "G-01", "fitness_current", "add", "9", "--root=$tmp"]) == M.EXIT_GUARD
    finally
        rm(tmp; recursive=true)
    end
end

@testset "cli: add artifact creates open artifact record" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "t", "--title=Codebase", "--root=$tmp", "--quiet"]) == 0
    st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
    @test haskey(st.nodes, "T-01")
    @test st.nodes["T-01"].kind === :t
    @test st.nodes["T-01"].status === :open
    rm(tmp; recursive=true)
end

@testset "cli: add question with targets expands to asks edges" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=Goal", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=W", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "q", "--cynefin=clear", "--targets=W-01",
                  "--title=Q", "--root=$tmp", "--quiet"]) == 0
    st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
    @test !haskey(st.nodes["Q-01"].fields, :targets)
    @test any(e -> e.label === :asks && e.from == "Q-01" && e.to == "W-01", st.edges)
    rm(tmp; recursive=true)
end

@testset "cli: init with stride allocates stepped padded ids on add work" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet", "--id-stride=4", "--id-offset=1"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=z", "--root=$tmp",
                      "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=q", "--root=$tmp",
                      "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"); verify=false)
        @test "W-001" in keys(st.nodes) && "W-005" in keys(st.nodes)
    finally
        rm(tmp; recursive=true)
    end
end

@testset "cli: renumber goal rewires goals field and fitness keys on work" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=X", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=T",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["fitness", "W-01", "G-01", "2", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["renumber", "G-01", "--root=$tmp", "--quiet", "--to=G-90"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test haskey(st.nodes, "G-90") && !haskey(st.nodes, "G-01")
        wg = collect(st.nodes["W-01"].fields[:goals])
        @test wg == ["G-90"]
        @test haskey(st.nodes["W-01"].fields[:fitness], "G-90")
    finally
        rm(tmp; recursive=true)
    end
end
