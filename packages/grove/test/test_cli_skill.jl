@testset "cli: skill print and install" begin
    tmp = mktempdir()
    try
        @test M.main(["skill", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["specification", "--root=$tmp", "--quiet"]) == 0
        out = joinpath(tmp, "skills")
        @test M.main(["skill", "--install=$out", "--root=$tmp", "--quiet"]) == 0
        installed = joinpath(out, "grove")
        skill = read(joinpath(installed, "SKILL.md"), String)
        @test occursin("](specification.md)", skill)
        @test !occursin("../../specification.md", skill)
        lines = split(skill, '\n')
        @test lines[1] == "---"
        @test startswith(lines[2], "version: ")
        specification = read(joinpath(@__DIR__, "..", "..", "..", "docs", "specification.md"), String)
        @test read(joinpath(installed, "specification.md"), String) == specification
        @test length(readdir(installed)) == 2
    finally
        rm(tmp; recursive=true, force=true)
    end
end
