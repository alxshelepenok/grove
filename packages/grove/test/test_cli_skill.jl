@testset "cli: skill print and install" begin
    tmp = mktempdir()
    try
        @test M.main(["skill", "--root=$tmp", "--quiet"]) == 0
        out = joinpath(tmp, "skills")
        @test M.main(["skill", "--install=$out", "--root=$tmp", "--quiet"]) == 0
        @test isfile(joinpath(out, "grove", "SKILL.md"))
        @test isfile(joinpath(out, "grove", "references", "rules.md"))
        @test isfile(joinpath(out, "grove", "diagrams", "workflow.md"))
        lines = split(read(joinpath(out, "grove", "SKILL.md"), String), '\n')
        @test lines[1] == "---"
        @test startswith(lines[2], "version: ")
        n = length(readdir(joinpath(out, "grove", "references")))
        @test n == 9
    finally
        rm(tmp; recursive=true, force=true)
    end
end
