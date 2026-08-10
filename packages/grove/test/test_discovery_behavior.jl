@testset "y status transition matrix" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")

        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-01"].status === :active
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-01"].status === :stale
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) != 0
        @test M.read_lock(lock).nodes["Y-01"].status === :stale
        @test M.main(["set", "Y-01", "status=superseded", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-01"].status === :superseded
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) != 0

        @test M.main(["add", "y", "--title=T2", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=stale", "--root=$tmp", "--quiet"]) != 0
        @test M.read_lock(lock).nodes["Y-02"].status === :proposed
        @test M.main(["set", "Y-02", "status=superseded", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-02"].status === :superseded

        @test M.main(["add", "y", "--title=T3", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "Y-03", "tags", "clear", "--root=$tmp", "--quiet"]) == 0
        err_path, err_io = mktemp()
        close(err_io)
        rc = Ref(-1)
        open(err_path, "w") do f
            redirect_stderr(f) do
                rc[] = M.main(["set", "Y-03", "status=active", "--root=$tmp"])
            end
        end
        etxt = read(err_path, String)
        rm(err_path, force=true)
        @test rc[] != 0
        @test occursin("I12", etxt)
        @test M.read_lock(lock).nodes["Y-03"].status === :proposed
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "decay on dead surface then cleared by stale" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0

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

        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["ok"] == true

        rm(joinpath(tmp, "src", "a.jl"))
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test d["ok"] == false
        @test any(e -> occursin("decay: Y-01", e) && occursin("dead surface", e), d["errors"])

        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["ok"] == true
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "decay on rotted distills origins" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "d", "--title=Ctx", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "b", "--title=B", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=TD", "--tags=auth", "--surface=src/a.jl",
                      "--from=D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=TB", "--tags=auth", "--surface=src/a.jl",
                      "--from=B-01", "--root=$tmp", "--quiet"]) == 0

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

        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0

        @test M.main(["set", "D-01", "status=superseded", "--root=$tmp", "--quiet"]) == 0
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test any(e -> occursin("decay: Y-01", e) && occursin("rotted origin", e), d["errors"])
        @test !any(e -> occursin("decay: Y-02", e), d["errors"])

        @test M.main(["set", "B-01", "status=invalidated_blocking", "--root=$tmp", "--quiet"]) == 0
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test any(e -> occursin("decay: Y-01", e), d["errors"])
        @test any(e -> occursin("decay: Y-02", e) && occursin("invalidated_blocking", e), d["errors"])

        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=stale", "--root=$tmp", "--quiet"]) == 0
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "decay skips rotted origins that are archived" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=W", "--root=$tmp", "--quiet"]) == 0
        for fn in ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=Ctx", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=ready", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=superseded", "--root=$tmp", "--quiet"]) == 0

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

        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test any(e -> occursin("decay: Y-01", e) && occursin("rotted origin", e), d["errors"])

        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["D-01"].archived
        @test st.nodes["D-01"].status === :superseded
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
        @test !any(e -> occursin("rotted origin", e), d["errors"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "decay on glossary term hand-removed from glossary.md" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        gpath = joinpath(tmp, ".grove", "glossary.md")
        open(gpath, "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0

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

        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0

        kept = [l for l in split(read(gpath, String), '\n') if !occursin("| auth |", l)]
        write(gpath, join(kept, "\n"))
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test any(e -> occursin("decay: Y-01", e) && occursin("lost glossary term", e) &&
                      occursin("auth", e), d["errors"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "revalidate pays with a fresh anchor" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        write(joinpath(tmp, "src", "b.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=Ctx", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")

        @test M.main(["revalidate", "Y-01", "--surface=src/b.jl", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["revalidate", "W-01", "--surface=src/b.jl", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["revalidate", "Y-99", "--surface=src/b.jl", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["revalidate", "Y-01", "--surface=src/b.jl", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["revalidate", "Y-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["revalidate", "Y-01", "--surface=src/missing.jl", "--root=$tmp", "--quiet"]) != 0
        @test M.read_lock(lock).nodes["Y-01"].status === :stale

        out_path, out_io = mktemp()
        close(out_io)
        rc = Ref(-1)
        open(out_path, "w") do f
            redirect_stdout(f) do
                rc[] = M.main(["revalidate", "Y-01", "--surface=src/b.jl", "--root=$tmp", "--json"])
            end
        end
        d = JSON.parse(read(out_path, String))
        rm(out_path, force=true)
        @test rc[] == 0
        @test d["command"] == "revalidate"
        @test d["id"] == "Y-01"
        st = M.read_lock(lock)
        x1 = st.nodes["Y-01"]
        @test x1.status === :active
        @test x1.fields[:surface] == ["src/b.jl"]
        @test length(x1.fields[:revalidation]) == 1
        @test occursin(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z surface=src/b\.jl$",
                       x1.fields[:revalidation][1])
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["revalidate", "Y-01", "--from=D-01", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(lock)
        @test st2.nodes["Y-01"].status === :active
        @test any(e -> e.label === :distills && e.from == "Y-01" && e.to == "D-01", st2.edges)
        @test length(st2.nodes["Y-01"].fields[:revalidation]) == 2
        @test occursin("from=D-01", st2.nodes["Y-01"].fields[:revalidation][2])

        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st3 = M.read_lock(lock)
        @test st3.nodes["Y-01"].status === :stale
        @test !any(e -> e.label === :distills && e.from == "Y-01" && e.to == "D-01", st3.edges)
        @test length(st3.nodes["Y-01"].fields[:revalidation]) == 1
        @test st3.nodes["Y-01"].fields[:surface] == ["src/b.jl"]

        @test M.main(["set", "D-01", "status=superseded", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["revalidate", "Y-01", "--from=D-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["revalidate", "Y-01", "--from=G-99", "--root=$tmp", "--quiet"]) != 0
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "glossary rename rewrites glossary.md and Discovery tags atomically" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        gpath = joinpath(tmp, ".grove", "glossary.md")
        open(gpath, "a") do io
            println(io, "| auth | a term | test |")
            println(io, "| seam | another term | test |")
        end
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth,seam", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")
        jpath = joinpath(tmp, ".grove", "journal.log")
        jbefore = length(readlines(jpath))

        @test M.main(["glossary", "rename", "auth", "identity", "--root=$tmp", "--quiet"]) == 0
        gtext = read(gpath, String)
        @test occursin("| identity |", gtext)
        @test !occursin("| auth |", gtext)
        @test occursin("| seam |", gtext)
        @test M.read_lock(lock).nodes["Y-01"].fields[:tags] == ["identity", "seam"]
        jlines = readlines(jpath)
        @test length(jlines) == jbefore + 1
        @test JSON.parse(jlines[end])["cmd"] == "glossary"
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["glossary", "rename", "identity", "seam", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["glossary", "rename", "nosuch", "term2", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["glossary", "rename", "seam", "seam", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["glossary", "bogus", "seam", "term3", "--root=$tmp", "--quiet"]) != 0

        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-01"].fields[:tags] == ["auth", "seam"]
        grestored = read(gpath, String)
        @test occursin("| auth |", grestored)
        @test !occursin("| identity |", grestored)
        @test occursin("| seam |", grestored)
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["add", "y", "--title=G", "--tags=ghost", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        out_path, out_io = mktemp()
        close(out_io)
        rc = Ref(-1)
        open(out_path, "w") do f
            redirect_stdout(f) do
                rc[] = M.main(["glossary", "rename", "ghost", "phantom", "--root=$tmp", "--json"])
            end
        end
        d = JSON.parse(read(out_path, String))
        rm(out_path, force=true)
        @test rc[] == 0
        @test d["command"] == "glossary"
        @test M.read_lock(lock).nodes["Y-02"].fields[:tags] == ["phantom"]
        @test !occursin("phantom", read(gpath, String))
        gbefore = read(gpath, String)
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["Y-02"].fields[:tags] == ["ghost"]
        @test read(gpath, String) == gbefore
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "session classification covers revalidate and glossary" begin
    @test "revalidate" in M.SESSION_MUTATE_COMMANDS
    @test !("revalidate" in M.SESSION_READ_COMMANDS)
    @test "glossary" in M.SESSION_MUTATE_COMMANDS
    @test !("glossary" in M.SESSION_READ_COMMANDS)
end
