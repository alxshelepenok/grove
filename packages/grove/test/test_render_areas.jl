function render_areas_fixture()
    st = M.State()
    z1 = M.Node(:a, "A-01"; title="Platform", status=:present)
    z1.fields[:surface] = ["src/platform.jl"]
    st.nodes["A-01"] = z1
    st.nodes["A-02"] = M.Node(:a, "A-02"; title="CLI", status=:present)
    st.nodes["A-03"] = M.Node(:a, "A-03"; title="Dormant", status=:present)

    g1 = M.Node(:g, "G-01"; title="Stable platform", status=:unverified)
    g1.fields[:area] = "A-01"
    g1.fields[:tags] = ["platform"]
    st.nodes["G-01"] = g1
    g2 = M.Node(:g, "G-02"; title="Polished CLI", status=:unverified)
    g2.fields[:area] = "A-02"
    st.nodes["G-02"] = g2

    w1 = M.Node(:w, "W-01"; title="Rework parser", type=:feature, status=:proposed, cynefin=:clear)
    w1.fields[:goals] = ["G-01"]
    w1.fields[:surface] = ["src/parser.jl"]
    w1.fields[:tags] = ["parser"]
    st.nodes["W-01"] = w1
    w2 = M.Node(:w, "W-02"; title="Polish repl", type=:feature, status=:ready, cynefin=:clear)
    w2.fields[:goals] = ["G-02"]
    w2.fields[:surface] = ["src/repl.jl"]
    w2.fields[:tags] = ["cli"]
    w2.fields[:fitness] = Dict("G-02" => 1)
    w2.fields[:ac] = ["a"]
    w2.fields[:hypothesis] = ["h"]
    w2.fields[:evidence_strategy] = ["e"]
    st.nodes["W-02"] = w2
    w3 = M.Node(:w, "W-03"; title="Shared plumbing", type=:feature, status=:proposed, cynefin=:clear)
    w3.fields[:goals] = ["G-01", "G-02"]
    st.nodes["W-03"] = w3
    st.nodes["W-04"] = M.Node(:w, "W-04"; title="Unscoped chore", type=:feature, status=:proposed, cynefin=:clear)

    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="Open parser question", status=:open, cynefin=:clear)
    st.nodes["Q-02"] = M.Node(:q, "Q-02"; title="Answered repl question", status=:answered, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="Validated parser bet", status=:validated, cynefin=:clear)
    st.nodes["B-02"] = M.Node(:b, "B-02"; title="Pending plumbing bet", status=:proposed, cynefin=:clear)
    st.nodes["D-01"] = M.Node(:d, "D-01"; title="Accepted repl decision", status=:accepted)
    push!(st.edges, M.Edge("Q-01", :asks, "W-01"))
    push!(st.edges, M.Edge("Q-02", :asks, "W-02"))
    push!(st.edges, M.Edge("B-01", :targets, "W-01"))
    push!(st.edges, M.Edge("B-02", :targets, "W-03"))
    push!(st.edges, M.Edge("W-02", :implements, "D-01"))

    x1 = M.Node(:y, "Y-01"; title="Parser lore", status=:active)
    x1.fields[:surface] = ["src/parser.jl"]
    x1.fields[:tags] = ["parser"]
    st.nodes["Y-01"] = x1
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    x2 = M.Node(:y, "Y-02"; title="Cross-area axiom", status=:active)
    x2.fields[:surface] = ["src/parser.jl", "src/repl.jl"]
    x2.fields[:tags] = ["shared"]
    st.nodes["Y-02"] = x2
    x3 = M.Node(:y, "Y-03"; title="Outdated parser note", status=:stale)
    x3.fields[:surface] = ["src/parser.jl"]
    x3.fields[:tags] = ["parser"]
    st.nodes["Y-03"] = x3
    x4 = M.Node(:y, "Y-04"; title="CLI process knowledge", status=:active)
    x4.fields[:tags] = ["cli"]
    x4.fields[:why] = ["No honest file anchor."]
    st.nodes["Y-04"] = x4
    st
end

@testset "areas: anchor sets derive from a surface, member goals and work" begin
    st = render_areas_fixture()
    z1 = st.nodes["A-01"]
    z2 = st.nodes["A-02"]
    @test M.area_surface(st, z1) == Set(["src/platform.jl", "src/parser.jl"])
    @test M.area_surface(st, z2) == Set(["src/repl.jl"])
    @test M.area_tags(st, z1) == Set(["platform", "parser"])
    @test M.area_tags(st, z2) == Set(["cli"])
    @test M.area_node_ids(st, z1) == Set(["G-01", "W-01", "W-03", "Q-01", "B-01", "B-02"])
    @test M.area_node_ids(st, z2) == Set(["G-02", "W-02", "W-03", "Q-02", "B-02", "D-01"])
end

@testset "areas: relevant Discoveries follow the soft tier, stale contributes zero" begin
    st = render_areas_fixture()
    z1 = st.nodes["A-01"]
    z2 = st.nodes["A-02"]
    z3 = st.nodes["A-03"]
    @test M.area_relevant_discoveries(st, z1) == ["Y-01", "Y-02"]
    @test M.area_relevant_discoveries(st, z2) == ["Y-02", "Y-04"]
    @test M.area_relevant_discoveries(st, z3) == String[]
    @test !("Y-03" in M.area_relevant_discoveries(st, z1))
    @test !("Y-03" in M.area_relevant_discoveries(st, z2))
end

@testset "render: Areas section shows per-area C and V as a relevance view" begin
    st = render_areas_fixture()
    md = M.render_index(st)
    @test occursin("## Areas", md)
    @test occursin("| Area | Title | C (content) | V (uncertainty) | Composition |", md)
    @test occursin("| A-01 | Platform | 3 | 4 | C: validated B 1 · answered Q 0 · accepted D 0 · active Discovery 2; V: open Q 1 · pending B 1 · W below DoR 2 |", md)
    @test occursin("| A-02 | CLI | 4 | 2 | C: validated B 0 · answered Q 1 · accepted D 1 · active Discovery 2; V: open Q 0 · pending B 1 · W below DoR 1 |", md)
    @test occursin("| A-03 | Dormant | 0 | 0 | C: validated B 0 · answered Q 0 · accepted D 0; V: open Q 0 · pending B 0 · W below DoR 0 |", md)
    @test occursin("| C (content) | 6 |", md)
    @test occursin("| V (uncertainty) | 5 |", md)
    @test occursin("not a partition", md)
    h1 = M.area_health(st, st.nodes["A-01"])
    h2 = M.area_health(st, st.nodes["A-02"])
    @test sum(values(h1.c)) + sum(values(h2.c)) > 6
    @test sum(values(h1.v)) + sum(values(h2.v)) > 5
end

@testset "render: a nodes carry the area mermaid class" begin
    st = render_areas_fixture()
    md = M.render_index(st)
    @test occursin("A_01[\"A-01: Platform\"]:::area", md)
    @test occursin("classDef area fill:#5a1e4a", md)
end
