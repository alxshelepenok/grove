@testset "dor: toggles false until mandatory fields filled then true" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["G-01"] = g
    w = M.Node(:w, "W-01"; title="x", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-01"] = w
    @test !M.dor(st, w)
    w.fields[:goals] = ["G-01"]
    w.fields[:ac] = ["a"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    w.fields[:fitness] = Dict("G-01" => 1)
    @test M.dor(st, w)
end

@testset "dor: bug spike and refactor conjuncts gate readiness independently" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:verified)
    base!(w::M.Node) = begin
        w.fields[:goals] = ["G-01"]
        w.fields[:ac] = ["a"]
        w.fields[:hypothesis] = ["h"]
        w.fields[:evidence_strategy] = ["e"]
        w.fields[:fitness] = Dict("G-01" => 1)
    end
    wb = M.Node(:w, "W-B"; title="b", type=:bug, status=:proposed, cynefin=:clear)
    base!(wb)
    st.nodes["W-B"] = wb
    @test !M.dor(st, wb)
    wb.fields[:repro] = ["repro steps"]
    @test M.dor(st, wb)

    ws = M.Node(:w, "W-S"; title="s", type=:spike, status=:proposed, cynefin=:complex)
    base!(ws)
    st.nodes["W-S"] = ws
    @test !M.dor(st, ws)
    ws.fields[:exit] = ["exit satisfied when D/Q/B recorded"]
    @test M.dor(st, ws)

    wr = M.Node(:w, "W-R"; title="r", type=:refactor, status=:proposed, cynefin=:clear)
    wr.fields[:goals] = ["G-01"]
    wr.fields[:ac] = ["a"]
    wr.fields[:evidence_strategy] = ["e"]
    wr.fields[:fitness] = Dict("G-01" => 1)
    st.nodes["W-R"] = wr
    a2 = M.Node(:t, "T-02"; title="ghost", status=:open)
    a2.archived = true
    st.nodes["T-02"] = a2
    push!(st.edges, M.Edge("T-02", :causes, "W-R"))
    @test !M.dor(st, wr)
    st.nodes["T-03"] = M.Node(:t, "T-03"; title="live", status=:open)
    push!(st.edges, M.Edge("T-03", :causes, "W-R"))
    @test M.dor(st, wr)
end

@testset "dor: each conjunct fails independently on synthetic feature refactor shapes" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:verified)
    gq = M.Node(:g, "G-07"; title="q", status=:verified)
    st.nodes["G-07"] = gq

    wf_base(wid, typ) =
        let w = M.Node(:w, wid; title = wid, type = typ, status = :proposed, cynefin = :clear)
            w.fields[:goals] = ["G-01"]
            w.fields[:ac] = ["ac"]
            w.fields[:hypothesis] = ["h"]
            w.fields[:evidence_strategy] = ["e"]
            w.fields[:fitness] = Dict("G-01" => 1)
            w
        end
    function conj_ok(w, label)::Bool
        for (lb, ok, _) in M.dor_breakdown(st, w)
            lb == label && return ok
        end
        error("missing conjunct $label")
    end

    w1 = wf_base("W-X1", :feature); st.nodes[w1.id] = w1
    w1.fields[:goals] = String[]
    @test conj_ok(w1, "goals(w) ≠ ∅") == false

    w2 = wf_base("W-X2", :feature); st.nodes[w2.id] = w2
    w2.fields[:ac] = String[]
    @test conj_ok(w2, "AC(w) ≠ ∅") == false

    w3 = wf_base("W-X3", :feature); st.nodes[w3.id] = w3
    st.nodes["Q-99"] = M.Node(:q, "Q-99"; title="q", status=:open, cynefin=:clear)
    push!(st.edges, M.Edge("Q-99", :asks, "W-X3"))
    @test conj_ok(w3, "∀ q ∈ asks(w), q terminal") == false
    st.nodes["Q-99"].status = :answered
    @test conj_ok(w3, "∀ q ∈ asks(w), q terminal")

    w4 = wf_base("W-X4", :feature); st.nodes[w4.id] = w4
    st.nodes["B-98"] =
        M.Node(:b, "B-98"; title="blk", status = :invalidated_blocking, cynefin = :clear)
    push!(st.edges, M.Edge("B-98", :targets, "W-X4"))
    @test conj_ok(w4, "BChain validated") == false

    w5 = wf_base("W-X5", :feature); st.nodes[w5.id] = w5
    w5.fields[:fitness] = Dict{String,Int}()
    @test conj_ok(w5, "fitness deltas set ∀ g") == false

    w6 = wf_base("W-X6", :feature); st.nodes[w6.id] = w6
    w6.fields[:evidence_strategy] = String[]
    @test conj_ok(w6, "evidence_strategy ≠ ∅") == false

    w7 = wf_base("W-X7", :feature); st.nodes[w7.id] = w7
    w7.fields[:hypothesis] = String[]
    @test conj_ok(w7, "hypothesis ≠ ⊥") == false

    w8 = wf_base("W-X8", :feature); st.nodes[w8.id] = w8
    w8.cynefin = :chaotic
    @test conj_ok(w8, "cynefin ≠ chaotic") == false

    wb = wf_base("W-XB", :bug); wb.fields[:repro] = String[]; st.nodes[wb.id] = wb
    @test conj_ok(wb, "repro(w) ≠ ∅") == false

    ws = wf_base("W-XS", :spike)
    ws.cynefin = :complex
    ws.fields[:exit] = String[]
    st.nodes[ws.id] = ws
    @test conj_ok(ws, "exit(w) ≠ ∅") == false

    wr = M.Node(:w, "W-XR"; title = "r", type = :refactor, status = :proposed,
                cynefin = :clear)
    wr.fields[:goals] = ["G-01"]; wr.fields[:ac] = ["a"]
    wr.fields[:evidence_strategy] = ["e"]; wr.fields[:fitness] = Dict("G-01" => 1)
    st.nodes[wr.id] = wr
    @test conj_ok(wr, "(A, causes, w) via materialised A") == false

    wg = wf_base("W-XM", :feature)
    wg.fields[:goals] = ["G-01", "G-07"]
    wg.fields[:fitness] = Dict("G-01" => 1)
    st.nodes[wg.id] = wg
    @test conj_ok(wg, "fitness deltas set ∀ g") == false
end
