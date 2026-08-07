@testset "algebra: bchain collects assumption linked by targets edge" begin
    st = M.State()
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:goals] = ["G-01"]; w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:ac] = ["x"]; w.fields[:hypothesis] = ["h"]; w.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = w
    b = M.Node(:b, "B-01"; title="b", status=:validated, cynefin=:clear)
    st.nodes["B-01"] = b
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    push!(st.edges, M.Edge("B-01", :targets, "W-01"))
    @test "B-01" in M.bchain(st, w)
end

@testset "algebra: bchain collects assumption via tests and question asks work" begin
    st = M.State()
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:goals] = ["G-01"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:ac] = ["x"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = w
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:answered, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="b", status=:validated, cynefin=:clear)
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    push!(st.edges, M.Edge("B-01", :tests, "Q-01"))
    push!(st.edges, M.Edge("Q-01", :asks, "W-01"))
    @test sort(M.bchain(st, w)) == ["B-01"]
end

@testset "algebra: refactor conjunct lists materialised artifacts sorted omitting archived" begin
    st = M.State()
    wr = M.Node(:w, "W-01"; title="r", type=:refactor, status=:ready, cynefin=:clear)
    wr.fields[:goals] = ["G-01"]
    wr.fields[:fitness] = Dict("G-01" => 1)
    wr.fields[:ac] = ["x"]
    wr.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = wr
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    for (id, arch) in (("T-02", false), ("T-01", false), ("T-09", true))
        a = M.Node(:t, id; title=id, status=:open)
        a.archived = arch
        st.nodes[id] = a
        arch || push!(st.edges, M.Edge(id, :causes, "W-01"))
    end
    ok, detail = M.refactor_materialised_root_cause(st, wr)
    @test ok
    @test detail == "T-01, T-02"
end

@testset "algebra: rederive opens artifact when no themed work remains" begin
    st = M.State()
    st.nodes["T-01"] = M.Node(:t, "T-01"; title="theme", status=:done)
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
    M.rederive_artifacts!(st)
    @test st.nodes["T-01"].status === :open
end

@testset "algebra: rederive closes artifact when all themed work terminal" begin
    st = M.State()
    st.nodes["T-01"] = M.Node(:t, "T-01"; title="Theme", status=:open)
    w1 = M.Node(:w, "W-01"; title="a", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:theme] = "T-01"
    st.nodes["W-01"] = w1
    w2 = M.Node(:w, "W-02"; title="b", type=:feature, status=:ready, cynefin=:clear)
    w2.fields[:theme] = "T-01"
    st.nodes["W-02"] = w2
    M.rederive_artifacts!(st)
    @test st.nodes["T-01"].status === :open
    w2.status = :done
    M.rederive_artifacts!(st)
    @test st.nodes["T-01"].status === :done
end

@testset "algebra: preds_clear requires verified goals on blocks edges" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:declined)
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:ready, cynefin=:clear)
    push!(st.edges, M.Edge("G-01", :blocks, "W-01"))
    @test !M.preds_clear(st, "W-01")
    st.nodes["G-01"].status = :verified
    @test M.preds_clear(st, "W-01")
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="x", type=:feature, status=:progress, cynefin=:clear)
    st.nodes["G-02"] = M.Node(:g, "G-02"; title="g2", status=:declined)
    push!(st.edges, M.Edge("G-02", :blocks, "W-02"))
    @test !isempty(M.i5_blocks_terminal(st))
    st.nodes["G-02"].status = :verified
    @test isempty(M.i5_blocks_terminal(st))
end

@testset "algebra: blocked_by deps impact critical_path and ready helpers" begin
    st = M.State()
    for (id, status) in (("W-01", :ready), ("W-02", :ready), ("W-03", :ready), ("W-04", :ready))
        n = M.Node(:w, id; title=id, type=:feature, status=status, cynefin=:clear)
        n.fields[:goals] = ["G-01"]
        n.fields[:fitness] = Dict("G-01" => 1)
        n.fields[:ac] = ["x"]
        n.fields[:hypothesis] = ["x"]
        n.fields[:evidence_strategy] = ["x"]
        st.nodes[id] = n
        M.record_id!(st, id)
    end
    g = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["G-01"] = g
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("W-02", :blocks, "W-03"))
    push!(st.edges, M.Edge("W-01", :blocks, "W-04"))
    @test "W-01" in M.blocked_by(st, "W-02")
    @test M.deps(st, "W-03") == ["W-01", "W-02"]
    @test sort(M.impact(st, "W-01")) == ["W-02", "W-03", "W-04"]
    cp = M.critical_path(st)
    @test cp == ["W-01", "W-02", "W-03"]
    rs = M.ready(st)
    @test "W-01" in [w.id for w in rs]
    @test !("W-02" in [w.id for w in rs])
end

@testset "algebra: renumber blocked when done evidence cites conflicting id tokens" begin
    st = M.State()
    wd = M.Node(:w, "W-99"; title="d", type=:feature, status=:done, cynefin=:clear)
    wd.fields[:evidence] = ["see W-01 and friends"]
    st.nodes["W-99"] = wd
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="other", type=:feature, status=:proposed,
                              cynefin=:clear)
    @test M.renumber_blocked_by_done_evidence(st, "W-01")
end
