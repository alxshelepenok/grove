@testset "invariants: produces edges reject invalid codomain types" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:ready, cynefin=:clear)
    st.nodes["B-01"] = M.Node(:b, "B-01"; title="b", status=:proposed, cynefin=:clear)
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:open, cynefin=:clear)
    st.nodes["D-01"] = M.Node(:d, "D-01"; title="d", status=:proposed)
    push!(st.edges, M.Edge("B-01", :targets, "W-01"))
    push!(st.edges, M.Edge("W-01", :produces, "Q-01"))
    push!(st.edges, M.Edge("W-01", :produces, "D-01"))
    @test isempty(M.check_edge_types(st))
    push!(st.edges, M.Edge("W-01", :produces, "W-01"))
    @test !isempty(M.check_edge_types(st))
end

@testset "invariants: done spike requires at least one produces edge" begin
    st = M.State()
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="s", type=:spike, status=:done, cynefin=:clear)
    errs = M.i2_spike_outputs(st)
    @test length(errs) == 1
    @test startswith(errs[1], "I2:")
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:open, cynefin=:clear)
    push!(st.edges, M.Edge("W-01", :produces, "Q-01"))
    @test isempty(M.i2_spike_outputs(st))
    st2 = M.State()
    st2.nodes["W-02"] = M.Node(:w, "W-02"; title="f", type=:feature, status=:done, cynefin=:clear)
    @test isempty(M.i2_spike_outputs(st2))
    st3 = M.State()
    st3.nodes["W-03"] = M.Node(:w, "W-03"; title="s", type=:spike, status=:proposed, cynefin=:clear)
    @test isempty(M.i2_spike_outputs(st3))
end
