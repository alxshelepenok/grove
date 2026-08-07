@testset "model exposes y kind statuses and distills label" begin
    @test :y in M.NODE_KINDS
    @test last(M.NODE_KINDS) === :a
    @test :distills in M.EDGE_LABELS
    @test M.STATUS[:y] == (:proposed, :active, :stale, :superseded)
    @test M.isterminal(:y, :superseded)
    @test !M.isterminal(:y, :active)
    @test !M.isterminal(:y, :stale)
end

@testset "lock roundtrip preserves y record and writes @grove 1 envelope" begin
    st = M.State()
    x = M.Node(:y, "Y-01"; title="Auth is separable", status=:active)
    x.fields[:tags] = ["auth", "seams"]
    x.fields[:surface] = ["src/auth.jl", "src/login.jl"]
    x.fields[:invariant] = ["module auth is separable behind interface Y"]
    x.fields[:why] = ["distilled from spike W-01"]
    x.fields[:skill_updates] = ["add auth checklist"]
    x.fields[:glossary_updates] = ["add term: seam"]
    x.fields[:revalidation] = ["2026-07-01: surface verified"]
    st.nodes["Y-01"] = x
    M.record_id!(st, "Y-01")
    w = M.Node(:w, "W-01"; title="Spike", type=:spike, status=:done, cynefin=:clear)
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    b = M.Node(:b, "B-01"; title="B", status=:validated, cynefin=:clear)
    st.nodes["B-01"] = b
    M.record_id!(st, "B-01")
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    push!(st.edges, M.Edge("Y-01", :distills, "B-01"))

    tmp = tempname()
    M.write_lock(tmp, st)
    txt = read(tmp, String)
    @test startswith(txt, "@grove 1\n")
    @test occursin("y Y-01 status=active", txt)

    st2 = M.read_lock(tmp)
    x2 = st2.nodes["Y-01"]
    @test x2.kind === :y
    @test x2.status === :active
    @test x2.title == "Auth is separable"
    @test x2.fields[:tags] == ["auth", "seams"]
    @test x2.fields[:surface] == ["src/auth.jl", "src/login.jl"]
    @test x2.fields[:invariant] == ["module auth is separable behind interface Y"]
    @test x2.fields[:why] == ["distilled from spike W-01"]
    @test x2.fields[:skill_updates] == ["add auth checklist"]
    @test x2.fields[:glossary_updates] == ["add term: seam"]
    @test x2.fields[:revalidation] == ["2026-07-01: surface verified"]
    @test any(e -> e.label === :produces && e.from == "W-01" && e.to == "Y-01", st2.edges)
    @test any(e -> e.label === :distills && e.from == "Y-01" && e.to == "B-01", st2.edges)
    rm(tmp)
end

@testset "y serializes after t before edges" begin
    st = M.State()
    st.nodes["T-01"] = M.Node(:t, "T-01"; title="theme", status=:open)
    st.nodes["Y-01"] = M.Node(:y, "Y-01"; title="da", status=:proposed)
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    M.record_id!(st, "T-01")
    M.record_id!(st, "Y-01")
    M.record_id!(st, "W-01")
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    body = M.serialize_body(st)
    pa = findfirst("t T-01", body)
    px = findfirst("y Y-01", body)
    pe = findfirst("e W-01", body)
    @test pa !== nothing && px !== nothing && pe !== nothing
    @test first(pa) < first(px)
    @test first(px) < first(pe)
end

@testset "add y acceptance matrix via CLI" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")
        @test split(read(lock, String), "\n")[1] == "@grove 1"

        @test M.main(["add", "d", "--title=Ctx", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["add", "y", "--title=T", "--surface=src/a.jl",
                      "--root=$tmp", "--quiet"]) != 0
        @test M.main(["add", "y", "--title=T", "--tags=auth",
                      "--root=$tmp", "--quiet"]) != 0
        @test M.main(["add", "y", "--tags=auth", "--surface=src/a.jl",
                      "--root=$tmp", "--quiet"]) != 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=",
                      "--root=$tmp", "--quiet"]) != 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--root=$tmp", "--quiet"]) != 0

        @test M.main(["add", "y", "--title=Surface Discovery", "--tags=auth",
                      "--surface=src/auth.jl", "--from=D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=Process Discovery", "--tags=auth",
                      "--why=process knowledge, no file anchor", "--from=D-01",
                      "--root=$tmp", "--quiet"]) == 0

        st = M.read_lock(lock)
        @test haskey(st.nodes, "Y-01")
        @test haskey(st.nodes, "Y-02")
        x1 = st.nodes["Y-01"]
        @test x1.status === :proposed
        @test x1.fields[:tags] == ["auth"]
        @test x1.fields[:surface] == ["src/auth.jl"]
        @test !haskey(st.nodes["Y-02"].fields, :surface)
        @test st.nodes["Y-02"].fields[:why] == ["process knowledge, no file anchor"]
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "add y --from wires produces and distills edges" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "b", "--title=B", "--cynefin=clear",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01,B-01", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")
        st = M.read_lock(lock)
        @test any(e -> e.label === :produces && e.from == "W-01" && e.to == "Y-01", st.edges)
        @test any(e -> e.label === :distills && e.from == "Y-01" && e.to == "B-01", st.edges)
        @test isempty(M.discovery_anchor_issues(st, st.nodes["Y-01"]))
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["add", "y", "--title=Bad", "--tags=auth", "--surface=src/a.jl",
                      "--from=G-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["add", "y", "--title=Bad2", "--tags=auth", "--surface=src/a.jl",
                      "--from=Q-99", "--root=$tmp", "--quiet"]) != 0
        st2 = M.read_lock(lock)
        @test !haskey(st2.nodes, "Y-02")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "link accepts y edge shapes and rejects illegal ones" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "b", "--title=B1", "--cynefin=clear",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "b", "--title=B2", "--cynefin=clear",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=One", "--tags=auth", "--surface=src/a.jl",
                      "--from=B-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=Two", "--tags=auth", "--surface=src/b.jl",
                      "--from=B-02", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["link", "W-01", "produces", "Y-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "Y-01", "distills", "B-02", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "Y-02", "supersedes", "Y-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "Y-01", "distills", "W-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["link", "B-01", "distills", "Y-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["link", "Y-01", "produces", "B-01", "--root=$tmp", "--quiet"]) != 0

        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test any(e -> e.label === :produces && e.from == "W-01" && e.to == "Y-01", st.edges)
        @test any(e -> e.label === :distills && e.from == "Y-01" && e.to == "B-02", st.edges)
        @test any(e -> e.label === :supersedes && e.from == "Y-02" && e.to == "Y-01", st.edges)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "discovery_anchor_issues flags anchor-less y and passes well-formed" begin
    st = M.State()
    x = M.Node(:y, "Y-01"; title="t", status=:proposed)
    st.nodes["Y-01"] = x
    issues = M.discovery_anchor_issues(st, x)
    @test length(issues) == 3
    @test all(i -> startswith(i, "I12:"), issues)
    @test length(M.check_all(st)) == 3

    x.fields[:tags] = ["auth"]
    @test length(M.discovery_anchor_issues(st, x)) == 2
    x.fields[:surface] = ["src/a.jl"]
    @test length(M.discovery_anchor_issues(st, x)) == 1
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:spike, status=:proposed, cynefin=:clear)
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    @test isempty(M.discovery_anchor_issues(st, x))
    @test isempty(M.check_all(st))

    st2 = M.State()
    x2 = M.Node(:y, "Y-01"; title="t", status=:proposed)
    x2.fields[:tags] = ["auth"]
    x2.fields[:why] = ["process knowledge"]
    st2.nodes["Y-01"] = x2
    st2.nodes["B-01"] = M.Node(:b, "B-01"; title="b", status=:validated, cynefin=:clear)
    push!(st2.edges, M.Edge("Y-01", :distills, "B-01"))
    @test isempty(M.discovery_anchor_issues(st2, x2))
    @test isempty(M.check_all(st2))

    w = M.Node(:w, "W-09"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    @test isempty(M.discovery_anchor_issues(st2, w))
end

@testset "check reports decay error on Discovery tags missing from glossary" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=unlisted", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")

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

        gpath = joinpath(tmp, ".grove", "glossary.md")
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test d["ok"] == false
        @test any(e -> occursin("decay: Y-01", e) && occursin("unlisted", e), d["errors"])

        open(gpath, "a") do io
            println(io, "| unlisted | a term | test |")
        end
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["ok"] == true
        @test isempty(d["errors"])

        rm(gpath)
        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc != 0
        @test any(e -> occursin("decay: Y-01", e), d["errors"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "render lists discoveries and discovery mermaid class" begin
    st = M.State()
    x = M.Node(:y, "Y-01"; title="Auth seam", status=:proposed)
    x.fields[:tags] = ["zeta", "alpha"]
    st.nodes["Y-01"] = x
    md = M.render_index(st)
    @test occursin("## Discoveries", md)
    @test occursin("| ID | Title | Tags | Status |", md)
    @test occursin("| Y-01 | Auth seam | alpha, zeta | proposed |", md)
    @test occursin("classDef discovery fill:#1f4e5f", md)
    @test occursin(":::discovery", md)
end

@testset "content health counts active Discovery only" begin
    st = M.State()
    xa = M.Node(:y, "Y-01"; title="a", status=:active)
    xp = M.Node(:y, "Y-02"; title="p", status=:proposed)
    st.nodes["Y-01"] = xa
    st.nodes["Y-02"] = xp
    h = M.content_health(st)
    @test h.c[:y] == 1
    md = M.render_index(st)
    @test occursin("active Discovery 1", md)
end

@testset "relevant_discoveries ranks surface-matching Discovery above tags-only" begin
    st = M.State()
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    w.fields[:surface] = ["src/auth.jl"]
    w.fields[:tags] = ["auth"]
    st.nodes["W-01"] = w
    x1 = M.Node(:y, "Y-01"; title="surface da", status=:active)
    x1.fields[:surface] = ["src/auth.jl"]
    x1.fields[:tags] = ["other"]
    st.nodes["Y-01"] = x1
    x2 = M.Node(:y, "Y-02"; title="tags da", status=:active)
    x2.fields[:tags] = ["auth"]
    st.nodes["Y-02"] = x2
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    r = M.relevant_discoveries(st, w, ["W-01"])
    @test r == ["Y-01", "Y-02"]
end

@testset "undo after add y removes node and its provenance edges" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=S",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")
        @test haskey(M.read_lock(lock).nodes, "Y-01")
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(lock)
        @test !haskey(st.nodes, "Y-01")
        @test !any(e -> e.from == "Y-01" || e.to == "Y-01", st.edges)
        @test haskey(st.nodes, "W-01")
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=T2", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(lock)
        @test haskey(st2.nodes, "Y-01")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "add y json mode emits id payload" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=Ctx", "--root=$tmp", "--quiet"]) == 0
        out_path, out_io = mktemp()
        close(out_io)
        rc = Ref(-1)
        open(out_path, "w") do f
            redirect_stdout(f) do
                rc[] = M.main(["add", "y", "--title=T", "--tags=auth",
                               "--surface=src/a.jl", "--from=D-01", "--root=$tmp", "--json"])
            end
        end
        d = JSON.parse(read(out_path, String))
        rm(out_path, force=true)
        @test rc[] == 0
        @test d["command"] == "add"
        @test d["kind"] == "y"
        @test d["id"] == "Y-01"
    finally
        rm(tmp; recursive=true, force=true)
    end
end
