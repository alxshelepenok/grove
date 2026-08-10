@testset "coverage: active_discovery_surfaces unions active Discovery surfaces only" begin
    st = M.State()
    xa = M.Node(:y, "Y-01"; title="a", status=:active)
    xa.fields[:surface] = ["src/a.jl", "src/b.jl"]
    st.nodes["Y-01"] = xa
    xs = M.Node(:y, "Y-02"; title="s", status=:stale)
    xs.fields[:surface] = ["src/stale.jl"]
    st.nodes["Y-02"] = xs
    xp = M.Node(:y, "Y-03"; title="p", status=:proposed)
    xp.fields[:surface] = ["src/proposed.jl"]
    st.nodes["Y-03"] = xp
    xd = M.Node(:y, "Y-04"; title="d", status=:superseded)
    xd.fields[:surface] = ["src/dead.jl"]
    st.nodes["Y-04"] = xd
    xn = M.Node(:y, "Y-05"; title="n", status=:active)
    st.nodes["Y-05"] = xn
    @test M.active_discovery_surfaces(st) == Set(["src/a.jl", "src/b.jl"])
end

@testset "coverage: ratio splits declared surface into covered and uncovered" begin
    st = M.State()
    xa = M.Node(:y, "Y-01"; title="a", status=:active)
    xa.fields[:surface] = ["src/a.jl"]
    st.nodes["Y-01"] = xa
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    st.nodes["W-01"] = w
    @test M.coverage(st, w) == (0.0, String[], String[])
    w.fields[:surface] = ["src/b.jl", "src/a.jl", "src/c.jl"]
    ratio, covered, uncovered = M.coverage(st, w)
    @test ratio ≈ 1 / 3
    @test covered == ["src/a.jl"]
    @test uncovered == ["src/b.jl", "src/c.jl"]
    xa.fields[:surface] = ["src/a.jl", "src/b.jl", "src/c.jl"]
    @test M.coverage(st, w)[1] ≈ 1.0
end

@testset "coverage: conjunct inactive by default and DoR unaffected" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:ac] = ["a"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:surface] = ["src/a.jl"]
    st.nodes["W-01"] = w
    @test M.dor(st, w)
    for (lb, ok, detail) in M.dor_breakdown(st, w)
        lb == "coverage(w) ≥ θ" || continue
        @test ok
        @test detail == "(coverage not required)"
    end
end

@testset "coverage: goal attr activates conjunct and active Discovery lifts it" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    g.attrs["requires_coverage"] = "true"
    st.nodes["G-01"] = g
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:ac] = ["a"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:surface] = ["src/a.jl", "src/b.jl", "src/c.jl"]
    st.nodes["W-01"] = w
    xa = M.Node(:y, "Y-01"; title="a", status=:active)
    xa.fields[:surface] = ["src/a.jl"]
    st.nodes["Y-01"] = xa
    @test !M.dor(st, w)
    det = ""
    for (lb, ok, detail) in M.dor_breakdown(st, w)
        lb == "coverage(w) ≥ θ" || continue
        @test !ok
        det = detail
    end
    @test det == "0.33 < 0.50; uncovered: src/b.jl, src/c.jl"
    xa.fields[:surface] = ["src/a.jl", "src/b.jl", "src/c.jl"]
    @test M.dor(st, w)
    for (lb, ok, detail) in M.dor_breakdown(st, w)
        lb == "coverage(w) ≥ θ" || continue
        @test ok
        @test detail == "1.00 ≥ 0.50"
    end
    xa.status = :stale
    @test !M.dor(st, w)
end

@testset "coverage: uncovered detail caps at five entries" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    g.attrs["requires_coverage"] = "0.9"
    st.nodes["G-01"] = g
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:surface] = ["src/$c.jl" for c in 'a':'h']
    st.nodes["W-01"] = w
    xa = M.Node(:y, "Y-01"; title="a", status=:active)
    xa.fields[:surface] = ["src/a.jl"]
    st.nodes["Y-01"] = xa
    det = ""
    for (lb, ok, detail) in M.dor_breakdown(st, w)
        lb == "coverage(w) ≥ θ" || continue
        @test !ok
        det = detail
    end
    @test det == "0.12 < 0.90; uncovered: src/b.jl, src/c.jl, src/d.jl, src/e.jl, src/f.jl … (+2 more)"
end

@testset "coverage: theta parsing and max over carriers" begin
    @test M.parse_requires_coverage("true") == 0.5
    @test M.parse_requires_coverage("0.3") == 0.3
    @test M.parse_requires_coverage("1") == 1.0
    @test M.parse_requires_coverage("abc") === nothing
    @test M.parse_requires_coverage("2") === nothing
    @test M.parse_requires_coverage("0") === nothing
    @test M.parse_requires_coverage("-0.5") === nothing
    @test M.parse_requires_coverage("") === nothing
    @test M.parse_requires_coverage(nothing) === nothing

    st = M.State()
    g1 = M.Node(:g, "G-01"; title="g1", status=:unverified)
    g1.attrs["requires_coverage"] = "0.3"
    st.nodes["G-01"] = g1
    g2 = M.Node(:g, "G-02"; title="g2", status=:unverified)
    g2.attrs["requires_coverage"] = "true"
    st.nodes["G-02"] = g2
    a = M.Node(:t, "T-01"; title="t", status=:open)
    a.attrs["requires_coverage"] = "0.7"
    st.nodes["T-01"] = a
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01", "G-02", "G-99"]
    st.nodes["W-01"] = w
    @test M.coverage_requirement(st, w) == 0.5
    w.fields[:theme] = "T-01"
    @test M.coverage_requirement(st, w) == 0.7
    w.fields[:goals] = String[]
    @test M.coverage_requirement(st, w) == 0.7
    w.fields[:theme] = "T-99"
    @test M.coverage_requirement(st, w) === nothing
end

@testset "coverage: theme-carried attr activates conjunct" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    a = M.Node(:t, "T-01"; title="t", status=:open)
    a.attrs["requires_coverage"] = "true"
    st.nodes["T-01"] = a
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:ac] = ["a"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:theme] = "T-01"
    w.fields[:surface] = ["src/a.jl"]
    st.nodes["W-01"] = w
    @test !M.dor(st, w)
end

@testset "coverage: none-form Discovery never counts" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    g.attrs["requires_coverage"] = "true"
    st.nodes["G-01"] = g
    x = M.Node(:y, "Y-01"; title="n", status=:active)
    x.fields[:why] = ["process knowledge"]
    st.nodes["Y-01"] = x
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:surface] = ["src/a.jl"]
    st.nodes["W-01"] = w
    @test isempty(M.active_discovery_surfaces(st))
    @test M.coverage(st, w)[1] == 0.0
end

@testset "coverage: empty declared surface fails with guidance" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    g.attrs["requires_coverage"] = "true"
    st.nodes["G-01"] = g
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:complex)
    w.fields[:goals] = ["G-01"]
    w.fields[:ac] = ["a"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    w.fields[:fitness] = Dict("G-01" => 1)
    st.nodes["W-01"] = w
    @test !M.dor(st, w)
    det = ""
    for (lb, ok, detail) in M.dor_breakdown(st, w)
        lb == "coverage(w) ≥ θ" || continue
        @test !ok
        det = detail
    end
    @test det == "no declared surface; declare via field W-01 surface add …"
end

@testset "coverage: non-feature and non-complex are exempt" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    g.attrs["requires_coverage"] = "true"
    st.nodes["G-01"] = g
    function base!(w)
        w.fields[:goals] = ["G-01"]
        w.fields[:ac] = ["a"]
        w.fields[:evidence_strategy] = ["e"]
        w.fields[:fitness] = Dict("G-01" => 1)
        st.nodes[w.id] = w
    end
    wb = M.Node(:w, "W-B"; title="b", type=:bug, status=:proposed, cynefin=:complex)
    base!(wb)
    wb.fields[:repro] = ["repro"]
    @test M.dor(st, wb)
    wf = M.Node(:w, "W-F"; title="f", type=:feature, status=:proposed, cynefin=:clear)
    base!(wf)
    wf.fields[:hypothesis] = ["h"]
    @test M.dor(st, wf)
    ws = M.Node(:w, "W-S"; title="s", type=:spike, status=:proposed, cynefin=:complex)
    base!(ws)
    ws.fields[:exit] = ["exit"]
    @test M.dor(st, ws)
    for w in (wb, wf, ws)
        for (lb, ok, detail) in M.dor_breakdown(st, w)
            lb == "coverage(w) ≥ θ" || continue
            @test ok
            @test detail == "(non-complex-feature)"
        end
    end
end

@testset "coverage cli: set requires_coverage validates and persists" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "t", "--title=T", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0

        @test M.main(["set", "G-01", "requires_coverage=abc", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "G-01", "requires_coverage=2", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "G-01", "requires_coverage=0", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "W-01", "requires_coverage=true", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["set", "G-01", "requires_coverage=0.6", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "T-01", "requires_coverage=true", "--root=$tmp", "--quiet"]) == 0

        lock = joinpath(tmp, ".grove", "state.lock")
        st = M.read_lock(lock)
        @test st.nodes["G-01"].attrs["requires_coverage"] == "0.6"
        @test st.nodes["T-01"].attrs["requires_coverage"] == "true"
        @test occursin("requires_coverage=0.6", read(lock, String))

        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st2 = M.read_lock(lock)
        @test !haskey(st2.nodes["T-01"].attrs, "requires_coverage")
        @test st2.nodes["G-01"].attrs["requires_coverage"] == "0.6"
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "coverage cli: dor conjunct fails then passes and progress is guarded" begin
    tmp = mktempdir()
    try
        function run_cmd(args)
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
            rc[], txt
        end

        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=complex", "--goals=G-01",
                      "--title=F", "--surface=src/a.jl,src/b.jl",
                      "--root=$tmp", "--quiet"]) == 0
        for fn in ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0

        lock = joinpath(tmp, ".grove", "state.lock")
        @test M.read_lock(lock).nodes["W-01"].fields[:surface] == ["src/a.jl", "src/b.jl"]

        rc, txt = run_cmd(["dor", "W-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("coverage(w) ≥ θ", txt)
        @test occursin("(coverage not required)", txt)
        @test occursin("result: ⊤", txt)

        @test M.main(["set", "G-01", "requires_coverage=true", "--root=$tmp", "--quiet"]) == 0
        rc, txt = run_cmd(["dor", "W-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("⊥  coverage(w) ≥ θ  → 0.00 < 0.50; uncovered: src/a.jl, src/b.jl", txt)
        @test occursin("result: ⊥", txt)
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) != 0

        @test M.main(["add", "y", "--title=Discovery", "--tags=auth", "--surface=src/a.jl,src/b.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        rc, txt = run_cmd(["dor", "W-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("⊤  coverage(w) ≥ θ  → 1.00 ≥ 0.50", txt)
        @test occursin("result: ⊤", txt)
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0

        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        rc, txt = run_cmd(["dor", "W-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("⊥  coverage(w) ≥ θ", txt)
        @test M.read_lock(lock).nodes["W-01"].status === :progress
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "coverage: in-flight progress keeps pinned DoR when Discovery goes stale" begin
    tmp = mktempdir()
    try
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

        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        write(joinpath(tmp, "src", "b.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--area=A-01", "--fitness-kind=manual", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "G-01", "requires_coverage=true", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=complex", "--goals=G-01",
                      "--title=F", "--surface=src/a.jl,src/b.jl",
                      "--root=$tmp", "--quiet"]) == 0
        for fn in ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=Discovery", "--tags=auth", "--surface=src/a.jl,src/b.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0

        lock = joinpath(tmp, ".grove", "state.lock")
        st = M.read_lock(lock)
        w1 = st.nodes["W-01"]
        @test M.dor(st, w1; pin_coverage=true)
        @test !M.dor(st, w1)
        for (lb, ok, detail) in M.dor_breakdown(st, w1; pin_coverage=true)
            lb == "coverage(w) ≥ θ" || continue
            @test ok
            @test detail == "(pinned at transition)"
        end

        rc, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test rc == 0
        @test !any(e -> occursin("I1", e), d["errors"])

        rc, d = run_json_cmd(["dor", "W-01", "--root=$tmp", "--json"])
        @test rc == 0
        @test d["dor"] == false

        @test M.main(["add", "w", "--type=feature", "--cynefin=complex", "--goals=G-01",
                      "--title=F2", "--surface=src/a.jl",
                      "--root=$tmp", "--quiet"]) == 0
        for fn in ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-02", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-02", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-02", "status=progress", "--root=$tmp", "--quiet"]) == M.EXIT_GUARD
    finally
        rm(tmp; recursive=true, force=true)
    end
end
