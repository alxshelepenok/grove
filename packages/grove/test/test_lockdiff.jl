@testset "lockdiff: ignores prose bullet permutation inside acceptance criteria" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="G", status=:unverified)
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="W", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:goals] = ["G-01"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:ac] = ["alpha", "bravo", "charlie"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    st2 = Base.deepcopy(st)
    st2.nodes["W-01"].fields[:ac] = ["charlie", "alpha", "bravo"]
    report = join(M.lock_structural_lines(st, st2), '\n')
    @test occursin("(no semantic changes)", report)
end

@testset "lockdiff: ignores permutation of edge declaration order" begin
    st = M.State()
    ga = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["G-01"] = ga
    M.record_id!(st, "G-01")
    wa = M.Node(:w, "W-01"; title="w1", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-01"] = wa
    wb = M.Node(:w, "W-02"; title="w2", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-02"] = wb
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("G-01", :blocks, "W-01"))
    sto = Base.deepcopy(st)
    reverse!(st.edges)
    @test sto.edges != st.edges
    report = join(M.lock_structural_lines(sto, st), '\n')
    @test occursin("(no semantic changes)", report)
end

@testset "lockdiff: title edits surface as semantic deltas" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="G", status=:unverified)
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="before", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:goals] = ["G-01"]
    w.fields[:fitness] = Dict("G-01" => 1)
    w.fields[:ac] = ["x"]
    w.fields[:hypothesis] = ["h"]
    w.fields[:evidence_strategy] = ["e"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    st2 = Base.deepcopy(st)
    st2.nodes["W-01"].title = "after"
    ls = M.lock_structural_lines(st, st2)
    @test any(l -> occursin(r"~ W-01", l), ls)
end

@testset "lockdiff: json payload marks no semantic change for identical deepcopy" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="G", status=:unverified)
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:ready, cynefin=:clear)
    w.fields[:goals] = ["G-01"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    sto = Base.deepcopy(st)
    pl = M.lock_structural_diff_payload(st, sto)
    @test pl["semantic_change"] == false
    @test isempty(pl["nodes"])
end
