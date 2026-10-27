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

function capture_cli(f)
    opath, oio = mktemp()
    close(oio)
    epath, eio = mktemp()
    close(eio)
    rc = -1
    open(opath, "w") do out
        open(epath, "w") do err
            redirect_stdout(out) do
                redirect_stderr(err) do
                    rc = f()
                end
            end
        end
    end
    result = (rc, read(opath, String), read(epath, String))
    rm(opath; force=true)
    rm(epath; force=true)
    result
end

@testset "cli failures emit diagnostics and success keeps quiet behavior" begin
    tmp = mktempdir()
    try
        rc, out, err = capture_cli(() -> M.main(["check", "--root=$tmp"]))
        @test rc == 1
        @test isempty(out)
        @test startswith(err, "error SYSTEM lock file is absent, initialize the project first\n  path=")
        M.validate_diag_text(split(err, '\n')[1][length("error SYSTEM ") + 1:end])

        @test capture_cli(() -> M.main(["init", "--root=$tmp", "--quiet"])) == (0, "", "")
        rc, out, err = capture_cli(() -> M.main(["init", "--root=$tmp"]))
        @test (rc, out) == (1, "")
        @test startswith(err, "error SYSTEM lock already exists\n  path=")

        rc, out, err = capture_cli(() -> M.main(["add", "w", "Positional", "--root=$tmp"]))
        @test (rc, out, err) == (1, "", "error INTERFACE add received unexpected positional argument Positional\n")

        rc, out, err = capture_cli(() -> M.main(["show", "W-99", "--root=$tmp"]))
        @test (rc, out, err) == (5, "", "error INTERFACE not found W-99\n")

        rc, out, err = capture_cli(() -> M.main(["deps", "--root=$tmp"]))
        @test (rc, out, err) == (1, "", "error INTERFACE deps requires a node id\n")

        rc, out, err = capture_cli(() -> M.main(["check", "--root=$tmp"]))
        @test (rc, out, err) == (0, "", "info OPERATIONAL ok\n")
    finally
        rm(tmp; recursive=true, force=true)
    end
end
