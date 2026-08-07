@testset "repo: gitattributes forces LF line endings on lock path" begin
    ga = joinpath(dirname(dirname(dirname(@__DIR__))), ".gitattributes")
    @test isfile(ga)
    txt = read(ga, String)
    @test occursin(".grove/state.lock", txt)
    @test occursin("eol=lf", txt)
end
