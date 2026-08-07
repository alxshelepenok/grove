include(joinpath(@__DIR__, "..", "conformance", "run.jl"))

@testset "conformance: corpus replays byte-identical" begin
    @test Conformance.verify_all()
end

@testset "conformance: normalization masks nondeterminism deterministically" begin
    paths = Tuple{String,String}[("C:\\tmp\\root9", "<root>"), ("C:\\tmp\\home9", "<home>")]
    tokens = ["host:0123456789abcdef"]
    raw = string("2026-07-19T02:40:01Z sha256:", repeat("a", 64), " ", repeat("b", 64),
                 " session=host:0123456789abcdef C:\\tmp\\root9\\.grove C:/tmp/root9/x\r\nkeep  \n")
    got = Conformance.normalize_text(raw, paths, tokens)
    @test got == "<ts> sha256:<sha> <sha> session=<session> <root>/.grove <root>/x\nkeep\n"
    @test Conformance.normalize_text(got, paths, tokens) == got
end
