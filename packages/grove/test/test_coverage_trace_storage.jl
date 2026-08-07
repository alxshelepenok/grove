@testset "coverage traces: relocate and materialize src round-trip" begin
    include(joinpath(@__DIR__, "..", "..", "..", "bin", "coverage", "trace_storage.jl"))
    mktempdir() do dir
        mkpath(joinpath(dir, "src", "sub"))
        mkpath(joinpath(dir, "test"))
        cov_src = joinpath(dir, "src", "sub", "b.jl.999.cov")
        cov_test = joinpath(dir, "test", "t.jl.1000.cov")
        write(cov_src, "SF:dummy\n")
        write(cov_test, "SF:dummy\n")

        relocate_cov_traces!(dir)
        @test !isfile(cov_src)
        @test !isfile(cov_test)
        @test isfile(joinpath(dir, "coverage", "traces", "src", "sub", "b.jl.999.cov"))
        @test isfile(joinpath(dir, "coverage", "traces", "test", "t.jl.1000.cov"))

        staged = materialize_src_traces!(dir)
        dest = joinpath(dir, "src", "sub", "b.jl.999.cov")
        @test sort(staged) == [dest]
        cleanup_materialized_src!(staged)
        @test !isfile(dest)
    end
end
