@testset "render: artifacts table lists causes and themed columns" begin
    st = M.State()
    a = M.Node(:t, "T-01"; title="Codebase", status=:open)
    w1 = M.Node(:w, "W-01"; title="U", type=:feature, status=:proposed, cynefin=:clear)
    w1.fields[:theme] = "T-01"
    w2 = M.Node(:w, "W-02"; title="V", type=:feature, status=:proposed, cynefin=:clear)
    b = M.Node(:b, "B-01"; title="Hyp", status=:proposed, cynefin=:clear)
    d = M.Node(:d, "D-01"; title="ADR", status=:proposed)
    q = M.Node(:q, "Q-01"; title="?", status=:open, cynefin=:clear)
    g = M.Node(:g, "G-01"; title="Goal", status=:unverified)
    st.nodes["T-01"] = a
    st.nodes["W-01"] = w1
    st.nodes["W-02"] = w2
    st.nodes["B-01"] = b
    st.nodes["D-01"] = d
    st.nodes["Q-01"] = q
    st.nodes["G-01"] = g
    push!(st.edges, M.Edge("T-01", :causes, "W-01"))
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("B-01", :targets, "W-02"))
    push!(st.edges, M.Edge("W-02", :produces, "D-01"))
    txt = M.render_index(st)
    @test occursin("## Themes", txt)
    @test occursin("| Causes work | Themed work |", txt)
    @test occursin("W-01", txt)
    @test occursin("==>|blocks|", txt)
    @test occursin("-.->|targets|", txt)
    @test occursin("-->|produces|", txt)
end

@testset "render: index lists sections tables and mermaid edge styles with critical path" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="Goal \"A\"", status=:unverified)
    st.nodes["G-01"].attrs["fitness"] = "1/1 x"
    st.nodes["D-01"] = M.Node(:d, "D-01"; title="Decide", status=:proposed)
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="Question", status=:open, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="Assume", status=:proposed, cynefin=:clear)
    st.nodes["T-01"] = M.Node(:t, "T-01"; title="Artifact", status=:open)
    st.nodes["Wf"] = M.Node(:w, "Wf"; title="Feat", type=:feature, status=:ready, cynefin=:clear)
    st.nodes["Ws"] = M.Node(:w, "Ws"; title="Spk", type=:spike, status=:proposed, cynefin=:clear)
    st.nodes["Wd"] = M.Node(:w, "Wd"; title="Done", type=:feature, status=:done, cynefin=:clear)
    st.nodes["Wp"] = M.Node(:w, "Wp"; title="Prog", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["Wrj"] = M.Node(:w, "Wrj"; title="No", type=:feature, status=:rejected, cynefin=:clear)
    for w in ("Wf", "Ws", "Wd", "Wp", "Wrj")
        st.nodes[w].fields[:goals] = ["G-01"]
        st.nodes[w].fields[:fitness] = Dict("G-01" => 1)
        st.nodes[w].fields[:ac] = ["a"]
        st.nodes[w].fields[:hypothesis] = ["h"]
        st.nodes[w].fields[:evidence_strategy] = ["e"]
    end
    st.nodes["Wd"].fields[:evidence] = ["done"]
    push!(st.edges, M.Edge("Q-01", :asks, "Wf"))
    push!(st.edges, M.Edge("B-01", :targets, "Wf"))
    push!(st.edges, M.Edge("B-01", :tests, "Q-01"))
    push!(st.edges, M.Edge("D-01", :supersedes, "D-01"))
    push!(st.edges, M.Edge("T-01", :causes, "Wf"))
    st.nodes["Wt"] = M.Node(:w, "Wt"; title="Themed", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["Wt"].fields[:goals] = ["G-01"]
    st.nodes["Wt"].fields[:theme] = "T-01"
    st.nodes["Wt"].fields[:fitness] = Dict("G-01" => 1)
    st.nodes["Wt"].fields[:ac] = ["a"]
    st.nodes["Wt"].fields[:hypothesis] = ["h"]
    st.nodes["Wt"].fields[:evidence_strategy] = ["e"]
    push!(st.edges, M.Edge("Wf", :blocks, "Wt"))
    md = M.render_index(st)
    @test occursin("## Goals", md)
    @test occursin("| G-01 |", md)
    @test occursin("## Decisions", md)
    @test occursin("## Open questions", md)
    @test occursin("| Q-01 |", md)
    @test occursin("## Assumptions", md)
    @test occursin("| B-01 |", md)
    @test occursin("Q-01", md) && occursin("Wf", md)
    @test occursin("## Themes", md)
    @test occursin("| T-01 |", md)
    @test occursin("==>|blocks|", md)
    @test occursin("-.->|targets|", md)
    @test occursin("-->|tests|", md)
    @test occursin("-->|asks|", md)
    @test occursin("-->|supersedes|", md)
    @test occursin("-->|causes|", md)
    @test occursin("class Wf", md) || occursin("Wf[", md)
    @test occursin(":::spike", md)
    @test occursin(":::done", md)
    @test occursin(":::progress", md)
    @test occursin(":::rejected", md)
    @test occursin(":::theme", md)
    @test occursin(":::goal", md)
    @test occursin("class Wf,Wt critical", md)
end

@testset "render: mermaid_safe maps hyphens and edge_line selects arrow style" begin
    @test M.mermaid_safe("W-01") == "W_01"
    @test M.mermaid_edge_line("A", "B", :blocks) == "  A ==>|blocks| B"
    @test M.mermaid_edge_line("A", "B", :targets) == "  A -.->|targets| B"
    @test M.mermaid_edge_line("A", "B", :implements) == "  A -->|implements| B"
end

@testset "render: content health counts C and V components" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="Goal", status=:unverified)
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="Open", status=:open, cynefin=:clear)
    st.nodes["Q-02"] = M.Node(:q, "Q-02"; title="Answered", status=:answered, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="Pending", status=:proposed, cynefin=:clear)
    st.nodes["B-02"] = M.Node(:b, "B-02"; title="Validated", status=:validated, cynefin=:clear)
    st.nodes["D-01"] = M.Node(:d, "D-01"; title="Accepted", status=:accepted)
    st.nodes["D-02"] = M.Node(:d, "D-02"; title="Proposed", status=:proposed)
    wok = M.Node(:w, "W-01"; title="Ready", type=:feature, status=:ready, cynefin=:clear)
    wok.fields[:goals] = ["G-01"]
    wok.fields[:fitness] = Dict("G-01" => 1)
    wok.fields[:ac] = ["a"]
    wok.fields[:hypothesis] = ["h"]
    wok.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = wok
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="Bare", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-03"] = M.Node(:w, "W-03"; title="Done", type=:feature, status=:done, cynefin=:clear)
    h = M.content_health(st)
    @test h.c == Dict(:b => 1, :q => 1, :d => 1, :y => 0)
    @test h.v == Dict(:q => 1, :b => 1, :w => 1)
    md = M.render_index(st)
    @test occursin("## Content health", md)
    @test occursin("| C (content) | 3 |", md)
    @test occursin("| V (uncertainty) | 3 |", md)
    @test occursin("validated B 1 · answered Q 1 · accepted D 1", md)
    @test occursin("open Q 1 · pending B 1 · W below DoR 1", md)
    @test !occursin("active Discovery", md)
end

@testset "render: content health V counts uncovered surface" begin
    st = M.State()
    w = M.Node(:w, "W-01"; title="U", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:surface] = ["src/a.jl"]
    st.nodes["W-01"] = w
    wd = M.Node(:w, "W-02"; title="D", type=:feature, status=:done, cynefin=:clear)
    wd.fields[:surface] = ["src/dead.jl"]
    st.nodes["W-02"] = wd
    wa = M.Node(:w, "W-03"; title="A", type=:feature, status=:ready, cynefin=:clear)
    wa.fields[:surface] = ["src/arch.jl"]
    wa.archived = true
    st.nodes["W-03"] = wa
    h = M.content_health(st)
    @test get(h.v, :surf, 0) == 1
    @test sum(values(h.v)) == 2
    md = M.render_index(st)
    @test occursin("W below DoR 1 · uncovered surface 1", md)
    @test occursin("| V (uncertainty) | 2 |", md)
    x = M.Node(:y, "Y-01"; title="x", status=:active)
    x.fields[:surface] = ["src/a.jl"]
    st.nodes["Y-01"] = x
    h2 = M.content_health(st)
    @test get(h2.v, :surf, 0) == 0
    @test sum(values(h2.v)) == 1
    @test !occursin("uncovered surface", M.render_index(st))
end

@testset "render: dashboard decay row appears only with decay signals" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        idx = joinpath(tmp, ".grove", "index.md")
        clean = read(idx, String)
        @test !occursin("Decay", clean)
        @test !occursin("uncovered surface", clean)
        @test M.main(["add", "w", "--type=spike", "--cynefin=clear", "--title=s",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=x", "--tags=foo", "--surface=dead/path.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        after = read(idx, String)
        @test occursin("| Decay | 1 |", after)
        @test M.main(["render", "--root=$tmp", "--quiet"]) == 0
        @test occursin("| Decay | 1 |", read(idx, String))
    finally
        rm(tmp; recursive=true, force=true)
    end
end
