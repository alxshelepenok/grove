@testset "archive: exclusive ids leave shared decision active across goals" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    g2 = M.Node(:g, "G-02"; title="b", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done, cynefin=:clear)
    w2 = M.Node(:w, "W-02"; title="w2", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01"]
    w2.fields[:goals] = ["G-02"]
    d1 = M.Node(:d, "D-01"; title="d", status=:accepted)
    st.nodes["G-01"] = g1
    st.nodes["G-02"] = g2
    st.nodes["W-01"] = w1
    st.nodes["W-02"] = w2
    st.nodes["D-01"] = d1
    push!(st.edges, M.Edge("W-01", :implements, "D-01"))
    push!(st.edges, M.Edge("W-02", :implements, "D-01"))
    ids = M.exclusive_archive_ids(st, "G-01")
    @test ids == Set(["G-01", "W-01"])
    @test !("D-01" in ids) && !("W-02" in ids) && !("G-02" in ids)
end

@testset "archive: exclusive closure pulls decision question chain when sole owner" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01"]
    d1 = M.Node(:d, "D-01"; title="d", status=:accepted)
    q1 = M.Node(:q, "Q-01"; title="q", status=:answered, cynefin=:clear)
    st.nodes["G-01"] = g1
    st.nodes["W-01"] = w1
    st.nodes["D-01"] = d1
    st.nodes["Q-01"] = q1
    push!(st.edges, M.Edge("W-01", :implements, "D-01"))
    push!(st.edges, M.Edge("Q-01", :asks, "W-01"))
    @test M.exclusive_archive_ids(st, "G-01") == Set(["G-01", "W-01", "D-01", "Q-01"])
end

@testset "archive: multi-goal work stays out of goal-exclusive closure" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    g2 = M.Node(:g, "G-02"; title="b", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01", "G-02"]
    st.nodes["G-01"] = g1
    st.nodes["G-02"] = g2
    st.nodes["W-01"] = w1
    @test M.exclusive_archive_ids(st, "G-01") == Set(["G-01"])
end

@testset "archive: discoveries stay out of the closure but satisfy the distill gate" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01"]
    d1 = M.Node(:d, "D-01"; title="d", status=:accepted)
    x1 = M.Node(:y, "Y-01"; title="da1", status=:active)
    x2 = M.Node(:y, "Y-02"; title="da2", status=:active)
    st.nodes["G-01"] = g1
    st.nodes["W-01"] = w1
    st.nodes["D-01"] = d1
    st.nodes["Y-01"] = x1
    st.nodes["Y-02"] = x2
    push!(st.edges, M.Edge("W-01", :implements, "D-01"))
    push!(st.edges, M.Edge("W-01", :produces, "Y-01"))
    push!(st.edges, M.Edge("Y-02", :distills, "D-01"))
    ids = M.exclusive_archive_ids(st, "G-01")
    @test ids == Set(["G-01", "W-01", "D-01"])
    @test "Y-01" ∉ ids && "Y-02" ∉ ids
    @test M.distill_linked_da_ids(st, ids) == ["Y-01", "Y-02"]
    @test isempty(M.distill_linked_da_ids(st, Set(["G-01"])))
end
