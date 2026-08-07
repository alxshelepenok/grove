function projects_capture(args; stderr_too::Bool=false)
    out_path, out_io = mktemp()
    close(out_io)
    rc = -1
    open(out_path, "w") do f
        if stderr_too
            redirect_stderr(f) do
                rc = M.main(args)
            end
        else
            redirect_stdout(f) do
                rc = M.main(args)
            end
        end
    end
    txt = read(out_path, String)
    rm(out_path; force=true)
    (rc, txt)
end

function projects_jlines(jp::AbstractString)
    isfile(jp) || return Dict{String,Any}[]
    [JSON.parse(l) for l in eachline(jp) if !isempty(strip(l))]
end

@testset "walk-up: command from a subdirectory resolves the enclosing project" begin
    tmp = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            sub = joinpath(tmp, "sub")
            mkpath(sub)
            @test M.main(["init", "--root=$sub", "--quiet"]) == 0
            @test M.main(["add", "d", "--title=Ctx", "--root=$sub", "--quiet"]) == 0
            @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                          "--from=D-01", "--root=$sub", "--quiet"]) == 0
            deeper = joinpath(sub, "deeper", "deeper2")
            mkpath(deeper)
            rc, txt = cd(deeper) do
                projects_capture(["list", "y"])
            end
            @test rc == 0
            @test occursin("Y-01", txt)
        finally
            rm(tmp; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "walk-up: no .grove/state.lock ancestor falls back to cwd" begin
    tmp = mktempdir()
    try
        @test M.walk_up_root(tmp) == abspath(tmp)
        mkpath(joinpath(tmp, ".grove"))
        deep = joinpath(tmp, "a", "b")
        mkpath(deep)
        @test M.walk_up_root(deep) == abspath(deep)
        write(joinpath(tmp, ".grove", "state.lock"), "")
        @test M.walk_up_root(deep) == abspath(tmp)
        @test M.walk_up_root(tmp) == abspath(tmp)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "--project and GROVE_PROJECT resolve to the right root" begin
    tmp = mktempdir()
    other = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh, "GROVE_PROJECT" => nothing) do
        try
            @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
            @test M.main(["init", "--root=$other", "--quiet"]) == 0
            @test M.main(["list", "y", "--project=$tmp"]) == 0
            rc, txt = projects_capture(["list", "y", "--project=$tmp"])
            @test rc == 0
            name = M.registry_name_for_path(M.registry_load(), tmp)
            @test name !== nothing
            @test M.main(["list", "y", "--project=$name"]) == 0
            rc = withenv("GROVE_PROJECT" => other) do
                M.main(["list", "y"])
            end
            @test rc == 0
            rc = withenv("GROVE_PROJECT" => name) do
                M.main(["list", "y"])
            end
            @test rc == 0
            @test M.main(["list", "y", "--project=no-such-project-name"]) == M.EXIT_NOTFOUND
            @test M.main(["list", "y", "--project=no-such-project-name", "--root=$tmp"]) == 0
        finally
            rm(tmp; recursive=true, force=true)
            rm(other; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "registry: unique suffixed names, created preserved, last_opened refreshed" begin
    p1 = joinpath(mktempdir(), "proj")
    p2 = joinpath(mktempdir(), "proj")
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            mkpath(p1)
            mkpath(p2)
            @test M.main(["init", "--root=$p1", "--quiet"]) == 0
            @test M.main(["init", "--root=$p2", "--quiet"]) == 0
            reg = M.registry_load()
            @test length(reg) == 2
            names = sort([e.name for e in reg])
            @test names == ["proj", "proj-2"]
            e1 = reg[findfirst(e -> e.path == abspath(p1), reg)]
            @test e1.name == "proj"
            @test !isempty(e1.created)
            @test e1.last_opened == e1.created

            M.registry_save([M.ProjectEntry(e1.name, e1.path, "1999-01-01T00:00:00Z",
                                            "2000-01-01T00:00:00Z"),
                             reg[findfirst(e -> e.path == abspath(p2), reg)]])
            @test M.main(["list", "y", "--root=$p1", "--quiet"]) == 0
            reg2 = M.registry_load()
            e2 = reg2[findfirst(e -> e.path == abspath(p1), reg2)]
            @test e2.created == "1999-01-01T00:00:00Z"
            @test e2.last_opened != "2000-01-01T00:00:00Z"
            @test length(reg2) == 2

            rc, txt = cd(gh) do
                projects_capture(["projects"])
            end
            @test rc == 0
            @test occursin("proj", txt)
            @test occursin(abspath(p1), txt)
            rc, txt = cd(gh) do
                projects_capture(["projects", "--json"])
            end
            @test rc == 0
            payload = JSON.parse(txt)
            @test payload["command"] == "projects"
            @test length(payload["projects"]) == 2
            row = payload["projects"][findfirst(r -> r["name"] == "proj", payload["projects"])]
            @test row["path"] == abspath(p1)
            @test row["created"] == "1999-01-01T00:00:00Z"
            @test haskey(row, "last_opened")
        finally
            rm(dirname(p1); recursive=true, force=true)
            rm(dirname(p2); recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "registry: malformed file warns but never crashes a command" begin
    tmp = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
            write(M.registry_path(), "[[projects]\nthis is not toml = = =\n")
            rc, err = cd(gh) do
                projects_capture(["projects"]; stderr_too=true)
            end
            @test rc == 0
            @test occursin("warning", err)
            rc, err = projects_capture(["list", "y", "--root=$tmp"]; stderr_too=true)
            @test rc == 0
            @test occursin("warning", err)
        finally
            rm(tmp; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "promote: copy with provenance, glossary backfill, journal, undo" begin
    src = mktempdir()
    dst = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            @test M.main(["init", "--root=$src", "--quiet"]) == 0
            @test M.main(["init", "--root=$dst", "--quiet"]) == 0
            @test M.main(["add", "d", "--title=Ctx", "--root=$src", "--quiet"]) == 0
            @test M.main(["add", "y", "--title=Auth Discovery", "--tags=auth,seams",
                          "--surface=src/auth.jl", "--from=D-01", "--root=$src", "--quiet"]) == 0
            @test M.main(["field", "Y-01", "invariant", "add", "auth is separable",
                          "--root=$src", "--quiet"]) == 0
            @test M.main(["field", "Y-01", "skill_updates", "add", "add auth checklist",
                          "--root=$src", "--quiet"]) == 0
            @test M.main(["field", "Y-01", "revalidation", "add", "2026-07-01: verified",
                          "--root=$src", "--quiet"]) == 0
            sst = M.read_lock(joinpath(src, ".grove", "state.lock"))
            src_version = sst.nodes["Y-01"].attrs["t_updated"]

            rc, txt = projects_capture(["promote", "Y-01", "--to=$dst",
                                        "--root=$src", "--json", "--quiet"])
            @test rc == 0
            payload = JSON.parse(txt)
            @test payload["command"] == "promote"
            @test payload["id"] == "Y-01"
            @test payload["origin_id"] == "Y-01"
            origin = payload["origin_project"]
            @test origin == M.registry_name_for_path(M.registry_load(), src)

            tst = M.read_lock(joinpath(dst, ".grove", "state.lock"))
            @test haskey(tst.nodes, "Y-01")
            x = tst.nodes["Y-01"]
            @test x.status === :proposed
            @test x.title == "Auth Discovery"
            @test String.(x.fields[:tags]) == ["auth", "seams"]
            @test String.(x.fields[:surface]) == ["src/auth.jl"]
            @test String.(x.fields[:invariant]) == ["auth is separable"]
            @test String.(x.fields[:skill_updates]) == ["add auth checklist"]
            @test !haskey(x.fields, :revalidation)
            @test x.attrs["origin_project"] == origin
            @test x.attrs["origin_id"] == "Y-01"
            @test x.attrs["origin_version"] == src_version
            @test isempty(tst.edges)

            gtxt = read(joinpath(dst, ".grove", "glossary.md"), String)
            @test occursin("| auth | copied from $origin |", gtxt)
            @test occursin("| seams | copied from $origin |", gtxt)
            @test Set(["auth", "seams"]) ⊆ M.glossary_terms(joinpath(dst, ".grove", "glossary.md"))

            recs = projects_jlines(joinpath(dst, ".grove", "journal.log"))
            @test recs[end]["cmd"] == "promote"
            @test recs[end]["inv"]["op"] == "rm_node"
            @test recs[end]["inv"]["id"] == "Y-01"
            srecs = projects_jlines(joinpath(src, ".grove", "journal.log"))
            @test !any(r -> get(r, "cmd", "") == "promote", srecs)

            @test M.main(["promote", "Y-01", "--to=$dst", "--root=$src", "--quiet"]) == M.EXIT_GUARD

            @test M.main(["undo", "--root=$dst", "--quiet"]) == 0
            tst2 = M.read_lock(joinpath(dst, ".grove", "state.lock"))
            @test !haskey(tst2.nodes, "Y-01")
            @test M.main(["check", "--root=$dst", "--quiet"]) == 0
        finally
            rm(src; recursive=true, force=true)
            rm(dst; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "promote: validation exits" begin
    src = mktempdir()
    dst = mktempdir()
    nolock = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            @test M.main(["init", "--root=$src", "--quiet"]) == 0
            @test M.main(["init", "--root=$dst", "--quiet"]) == 0
            @test M.main(["add", "d", "--title=Ctx", "--root=$src", "--quiet"]) == 0
            @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                          "--from=D-01", "--root=$src", "--quiet"]) == 0
            @test M.main(["promote", "Y-01", "--root=$src", "--quiet"]) == M.EXIT_ERR
            @test M.main(["promote", "D-01", "--to=$dst", "--root=$src", "--quiet"]) == M.EXIT_ERR
            @test M.main(["promote", "Y-99", "--to=$dst", "--root=$src", "--quiet"]) == M.EXIT_NOTFOUND
            @test M.main(["promote", "Y-01", "--to=no-such-project-name",
                          "--root=$src", "--quiet"]) == M.EXIT_NOTFOUND
            @test M.main(["promote", "Y-01", "--to=$nolock", "--root=$src", "--quiet"]) == M.EXIT_ERR
            @test M.main(["promote", "Y-01", "--to=$src", "--root=$src", "--quiet"]) == M.EXIT_ERR
        finally
            rm(src; recursive=true, force=true)
            rm(dst; recursive=true, force=true)
            rm(nolock; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end

@testset "promote: target resolvable by registry name" begin
    src = mktempdir()
    dst = mktempdir()
    gh = mktempdir()
    withenv("GROVE_HOME" => gh) do
        try
            @test M.main(["init", "--root=$src", "--quiet"]) == 0
            @test M.main(["init", "--root=$dst", "--quiet"]) == 0
            @test M.main(["add", "d", "--title=Ctx", "--root=$src", "--quiet"]) == 0
            @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                          "--from=D-01", "--root=$src", "--quiet"]) == 0
            dname = M.registry_name_for_path(M.registry_load(), dst)
            @test dname !== nothing
            @test M.main(["promote", "Y-01", "--to=$dname", "--root=$src", "--quiet"]) == 0
            tst = M.read_lock(joinpath(dst, ".grove", "state.lock"))
            @test haskey(tst.nodes, "Y-01")
            @test tst.nodes["Y-01"].attrs["origin_project"] ==
                  M.registry_name_for_path(M.registry_load(), src)
        finally
            rm(src; recursive=true, force=true)
            rm(dst; recursive=true, force=true)
            rm(gh; recursive=true, force=true)
        end
    end
end
