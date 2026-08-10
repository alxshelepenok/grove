function areas_capture_err(args)
    err_path, err_io = mktemp()
    close(err_io)
    rc = -1
    open(err_path, "w") do f
        redirect_stderr(f) do
            rc = M.main(args)
        end
    end
    txt = read(err_path, String)
    rm(err_path; force=true)
    (rc, txt)
end

function areas_run_json(args)
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

function areas_write_lock_fixture(path::AbstractString, magic::AbstractString, body::AbstractString)
    write(path, join([magic, "# AUTO-GENERATED. Do not edit. Use `grove` CLI.",
                      "# checksum: sha256:" * M.checksum_of(body), "", body], "\n"))
end

@testset "areas: model exposes a kind with structural status" begin
    @test :a in M.NODE_KINDS
    @test last(M.NODE_KINDS) === :a
    @test M.STATUS[:a] == (:present,)
    @test !M.isterminal(:a, :present)
    @test M.FAMILY_PREFIX[:a] == 'A'
    st = M.State()
    @test M.next_id!(st, :a) == "A-01"
    @test M.next_id!(st, :a) == "A-02"
end

@testset "areas: a and g grammar roundtrip" begin
    st = M.State()
    z1 = M.Node(:a, "A-01"; title="Platform", status=:present)
    z1.fields[:surface] = ["src/a.jl", "src/b.jl"]
    st.nodes["A-01"] = z1
    M.record_id!(st, "A-01")
    z2 = M.Node(:a, "A-02"; title="Bare", status=:present)
    st.nodes["A-02"] = z2
    M.record_id!(st, "A-02")
    g = M.Node(:g, "G-01"; title="Goal", status=:unverified)
    g.fields[:area] = "A-01"
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")

    tmp = tempname()
    M.write_lock(tmp, st)
    txt = read(tmp, String)
    @test startswith(txt, "@grove 1\n")
    @test occursin("a A-01 status=present", txt)
    @test occursin("surface: src/a.jl, src/b.jl", txt)
    @test occursin("area: A-01", txt)

    st2 = M.read_lock(tmp)
    r1 = st2.nodes["A-01"]
    @test r1.kind === :a
    @test r1.status === :present
    @test r1.title == "Platform"
    @test r1.fields[:surface] == ["src/a.jl", "src/b.jl"]
    r2 = st2.nodes["A-02"]
    @test !haskey(r2.fields, :surface)
    @test st2.nodes["G-01"].fields[:area] == "A-01"
    @test isempty(M.check_all(st2))
    rm(tmp)
end

@testset "areas: add a via CLI with and without surface" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        rc, etxt = areas_capture_err(["add", "a", "--root=$tmp", "--quiet"])
        @test rc != 0
        @test occursin("add a: --title is required", etxt)
        @test M.main(["add", "a", "--title=Platform", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=CLI", "--surface=src/a.jl,src/b.jl",
                      "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["A-01"].kind === :a
        @test st.nodes["A-01"].status === :present
        @test !haskey(st.nodes["A-01"].fields, :surface)
        @test st.nodes["A-02"].fields[:surface] == ["src/a.jl", "src/b.jl"]
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "areas: add g requires --area referencing an existing area" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Platform", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--title=W", "--root=$tmp", "--quiet"]) == 0

        rc, etxt = areas_capture_err(["add", "g", "--title=NoArea", "--root=$tmp", "--quiet"])
        @test rc == M.EXIT_ERR
        @test occursin("add g: --area=A-NN is required", etxt)

        rc2, etxt2 = areas_capture_err(["add", "g", "--title=BadArea", "--area=A-99",
                                        "--root=$tmp", "--quiet"])
        @test rc2 == M.EXIT_ERR
        @test occursin("add g: unknown --area id: A-99", etxt2)

        rc3, _ = areas_capture_err(["add", "g", "--title=NotArea", "--area=W-01",
                                    "--root=$tmp", "--quiet"])
        @test rc3 == M.EXIT_ERR

        @test M.main(["add", "g", "--title=Ok", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].fields[:area] == "A-01"
        @test !haskey(st.nodes, "G-02")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "areas: set area re-partitions goal membership with undo" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Platform", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=CLI", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        lock = joinpath(tmp, ".grove", "state.lock")

        @test M.main(["set", "G-01", "area=A-02", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["G-01"].fields[:area] == "A-02"

        rc, etxt = areas_capture_err(["set", "G-01", "area=A-99", "--root=$tmp", "--quiet"])
        @test rc != 0
        @test occursin("set: unknown area: A-99", etxt)
        @test M.read_lock(lock).nodes["G-01"].fields[:area] == "A-02"

        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        @test M.read_lock(lock).nodes["G-01"].fields[:area] == "A-01"
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "areas: a status cannot be changed" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Platform", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "A-01", "status=present", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "A-01", "status=active", "--root=$tmp", "--quiet"]) != 0
        @test M.read_lock(joinpath(tmp, ".grove", "state.lock")).nodes["A-01"].status === :present
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "areas: check_area_membership flags missing and dangling area" begin
    st = M.State()
    st.nodes["A-01"] = M.Node(:a, "A-01"; title="Platform", status=:present)
    g1 = M.Node(:g, "G-01"; title="ok", status=:unverified)
    g1.fields[:area] = "A-01"
    st.nodes["G-01"] = g1
    @test isempty(M.check_area_membership(st))

    st.nodes["G-02"] = M.Node(:g, "G-02"; title="missing", status=:unverified)
    g3 = M.Node(:g, "G-03"; title="dangling", status=:unverified)
    g3.fields[:area] = "A-99"
    st.nodes["G-03"] = g3
    g4 = M.Node(:g, "G-04"; title="archived", status=:verified)
    g4.archived = true
    st.nodes["G-04"] = g4
    msgs = M.check_area_membership(st)
    @test length(msgs) == 3
    @test all(m -> startswith(m, "I13:"), msgs)
    @test any(m -> occursin("G-02", m) && occursin("no `area`", m), msgs)
    @test any(m -> occursin("G-03", m) && occursin("A-99", m), msgs)
    @test any(m -> occursin("G-04", m), msgs)
end

@testset "areas: @grove 1 lock with area-less goal is I13 and never silently repaired" begin
    tmp = mktempdir()
    try
        mkpath(joinpath(tmp, ".grove"))
        lock = joinpath(tmp, ".grove", "state.lock")
        body = "g G-01 status=unverified \"Hand edited\""
        areas_write_lock_fixture(lock, "@grove 1", body)

        rc, d = areas_run_json(["check", "--root=$tmp", "--json"])
        @test rc == M.EXIT_INVARIANT
        @test d["ok"] == false
        @test any(e -> startswith(e, "I13:") && occursin("G-01", e), d["errors"])

        rc2, etxt2 = areas_capture_err(["add", "w", "--title=W", "--root=$tmp", "--quiet"])
        @test rc2 == 0
        @test !occursin("migrated:", etxt2)
        st = M.read_lock(lock)
        @test !haskey(st.nodes, "A-01")
        @test !haskey(st.nodes["G-01"].fields, :area)

        rc3, d3 = areas_run_json(["check", "--root=$tmp", "--json"])
        @test rc3 == M.EXIT_INVARIANT
        @test any(e -> startswith(e, "I13:"), d3["errors"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end
