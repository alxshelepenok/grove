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

@testset "conformance: diagnostic validators accept only four categories" begin
    for code in string.(M.DIAG_CODES)
        d = Dict("severity" => "error", "code" => code, "text" => "message")
        @test Conformance.validate_diag_object(d)
        Conformance.validate_json_diagnostics(d)
        Conformance.validate_rendered_diagnostics("error $code message\n")
    end
    for legacy in ("GENERAL", "OTHER", "INFO", "ERR", "NOT_FOUND", "DOR_NOT_SATISFIED")
        d = Dict("severity" => "error", "code" => legacy, "text" => "message")
        @test_throws ErrorException Conformance.validate_diag_object(d)
        @test_throws ErrorException Conformance.validate_rendered_diagnostics("error $legacy message\n")
    end
    @test !Conformance.validate_diag_object(Dict("command" => "ready", "items" => []))
    Conformance.validate_rendered_diagnostics("grove help and documents: remain unchanged\n")
    Conformance.validate_json_output_diagnostics("not JSON")
end
