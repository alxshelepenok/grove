@testset "diagnostic codes form a closed four-category set" begin
    @test length(M.DIAG_CODES) == 4
    @test collect(string.(M.DIAG_CODES)) == ["SYSTEM", "INTERFACE", "INVARIANT", "OPERATIONAL"]
    for code in M.DIAG_CODES
        @test M.diag_code_from_name(string(code)) === code
    end
    for bad in ("", "SYSTEM ", "system", "GENERAL", "OTHER", "INFO", "ERR", "USAGE",
                "GUARD", "NOT_FOUND", "CHECKSUM", "IO", "NODE_UNKNOWN",
                "I9_ASSUMPTION_CHAIN", "LOCK_MISSING", "CHECK_OK", "UNDO_NO_JOURNAL",
                "REVALIDATE_STATUS", "DECAY_ORIGIN_ROTTED", "SESSION_LOCK_TIMEOUT",
                "GOAL_VERIFIED_DISTILL", "UNKNOWN_COMMAND", "DOR_NOT_SATISFIED")
        @test M.diag_code_from_name(bad) === nothing
        @test_throws ArgumentError M.DiagCode(String(bad))
    end
end

@testset "diagnostic constructors keep severity independent from category" begin
    cases = (
        (M.system_error("disk failure"), "error", M.CODE_SYSTEM),
        (M.system_warning("registry malformed"), "warning", M.CODE_SYSTEM),
        (M.interface_error("unknown command"), "error", M.CODE_INTERFACE),
        (M.invariant_error("terminal record is frozen"), "error", M.CODE_INVARIANT),
        (M.operational_info("initialised"), "info", M.CODE_OPERATIONAL),
        (M.operational_warning("nothing to undo"), "warning", M.CODE_OPERATIONAL),
    )
    for (d, severity, code) in cases
        @test d.severity == severity
        @test d.code === code
    end
    for code in M.DIAG_CODES, (severity, ctor) in (("error", M.diag_error), ("warning", M.diag_warning), ("info", M.diag_info))
        @test ctor(code, "text").severity == severity
    end
    @test_throws ArgumentError M.raw_diag("fatal", M.CODE_SYSTEM, "raw text")
end

@testset "diagnostic text validation remains active" begin
    for bad in ("bad; text", "bad: text", "bad\u2014text")
        @test_throws ArgumentError M.interface_error(bad)
    end
    @test_throws ArgumentError M.interface_error("   ")
    for text in ("run `grove undo` to recover", "see grove dor W-01 for details",
                 "inspect with grove check first", "grove init the project")
        @test_throws ArgumentError M.interface_error(text)
    end
    @test M.interface_error("the Grove skill documents the Delivery loop").code === M.CODE_INTERFACE
end

@testset "diagnostic rendering and JSON use canonical categories" begin
    d = M.invariant_error("node is terminal")
    @test M.render_diag(d) == "error INVARIANT node is terminal"
    @test M.render_diag(M.system_warning("careful")) == "warning SYSTEM careful"
    @test M.render_diag(M.operational_info("note")) == "info OPERATIONAL note"

    detailed = M.system_error("lock file is absent, initialize the project first",
        Pair{String,String}["path" => "Y:/tmp/proj/.grove/state.lock", "attempt" => "2"])
    @test M.render_diag(detailed) ==
          "error SYSTEM lock file is absent, initialize the project first\n  path=Y:/tmp/proj/.grove/state.lock\n  attempt=2"
    parsed = JSON.parse(M.diag_json_str(detailed))
    @test parsed["severity"] == "error"
    @test parsed["code"] == "SYSTEM"
    @test parsed["text"] == "lock file is absent, initialize the project first"
    @test parsed["details"]["path"] == "Y:/tmp/proj/.grove/state.lock"
    @test parsed["details"]["attempt"] == "2"

    plain = JSON.parse(M.diag_json_str(M.interface_error("plain message")))
    @test plain == Dict("severity" => "error", "code" => "INTERFACE", "text" => "plain message")
end
