@testset "goal_fitness: count kind verifies when contributions match target" begin
    let st = M.State()
        g = M.Node(:g, "G-01"; title="g", status=:unverified)
        g.attrs["fitness_kind"] = "count"
        g.fields[:fitness_target] = "5"
        st.nodes["G-01"] = g
        w = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-01"]
        w.fields[:fitness] = Dict("G-01" => 5)
        st.nodes["W-01"] = w
        M.record_id!(st, "G-01")
        M.record_id!(st, "W-01")
        M.refresh_goal_structured_fitness!(st, g)
        @test g.status === :verified
        @test g.fields[:fitness_current] == "5"
    end
    let st = M.State()
        g = M.Node(:g, "G-02"; title="g", status=:unverified)
        g.attrs["fitness_kind"] = "manual"
        st.nodes["G-02"] = g
        w = M.Node(:w, "W-02"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-02"]
        w.fields[:fitness] = Dict("G-02" => 99)
        st.nodes["W-02"] = w
        M.record_id!(st, "G-02")
        M.record_id!(st, "W-02")
        M.refresh_goal_structured_fitness!(st, g)
        @test g.status === :unverified
        @test !haskey(g.fields, :fitness_current)
    end
    let st = M.State()
        g = M.Node(:g, "G-03"; title="g", status=:unverified)
        g.attrs["fitness"] = "3/3 x"
        st.nodes["G-03"] = g
        w = M.Node(:w, "W-03"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-03"]
        w.fields[:fitness] = Dict("G-03" => 3)
        st.nodes["W-03"] = w
        M.record_id!(st, "G-03")
        M.record_id!(st, "W-03")
        M.refresh_goal_structured_fitness!(st, g)
        @test g.status === :verified
    end
    let st = M.State()
        g = M.Node(:g, "G-04"; title="b", status=:unverified)
        g.attrs["fitness_kind"] = "boolean"
        st.nodes["G-04"] = g
        w = M.Node(:w, "W-04"; title="b", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-04"]
        w.fields[:fitness] = Dict("G-04" => 1)
        st.nodes["W-04"] = w
        M.record_id!(st, "G-04")
        M.record_id!(st, "W-04")
        M.refresh_goal_structured_fitness!(st, g)
        @test g.status === :verified
        @test g.fields[:fitness_current] == "true"
    end
    st = M.State()
    g = M.Node(:g, "G-99"; title="t", status=:unverified)
    g.attrs["fitness_kind"] = "metric"
    g.fields[:fitness_target] = "10"
    g.fields[:fitness_current] = "0"
    st.nodes["G-99"] = g
    M.record_id!(st, "G-99")
    tmp = tempname()
    M.write_lock(tmp, st)
    st2 = M.read_lock(tmp; verify=false)
    @test st2.nodes["G-99"].attrs["fitness_kind"] == "metric"
    @test st2.nodes["G-99"].fields[:fitness_target] == "10"
    rm(tmp)
end

@testset "goal_fitness: rederive runs on non-done transitions and demotes stale verified" begin
    # contributor rejection drops the aggregate and demotes the verified goal
    let st = M.State()
        g = M.Node(:g, "G-01"; title="g", status=:unverified)
        g.attrs["fitness_kind"] = "count"
        g.fields[:fitness_target] = "1"
        st.nodes["G-01"] = g
        w = M.Node(:w, "W-01"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-01"]
        w.fields[:fitness] = Dict("G-01" => 1)
        st.nodes["W-01"] = w
        M.record_id!(st, "G-01")
        M.record_id!(st, "W-01")
        M.rederive_goals!(st, w)
        @test g.status === :verified
        # the done contributor is rejected: status mutation re-derives despite non-done state
        w.status = :rejected
        M.rederive_goals!(st, w)
        @test g.fields[:fitness_current] == "0"
        @test g.status === :unverified
    end
    # below-target aggregate demotes verified to partial, not further
    let st = M.State()
        g = M.Node(:g, "G-02"; title="g", status=:unverified)
        g.attrs["fitness_kind"] = "count"
        g.fields[:fitness_target] = "3"
        st.nodes["G-02"] = g
        w = M.Node(:w, "W-02"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-02"]
        w.fields[:fitness] = Dict("G-02" => 3)
        st.nodes["W-02"] = w
        M.record_id!(st, "G-02")
        M.record_id!(st, "W-02")
        M.rederive_goals!(st, w)
        @test g.status === :verified
        w.fields[:fitness] = Dict("G-02" => 1)
        M.rederive_goals!(st, w)
        @test g.status === :partial
    end
    # declined goals are never overwritten by the derivation
    let st = M.State()
        g = M.Node(:g, "G-03"; title="g", status=:declined)
        g.attrs["fitness_kind"] = "count"
        g.fields[:fitness_target] = "1"
        st.nodes["G-03"] = g
        w = M.Node(:w, "W-03"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-03"]
        w.fields[:fitness] = Dict("G-03" => 0)
        st.nodes["W-03"] = w
        M.record_id!(st, "G-03")
        M.record_id!(st, "W-03")
        M.rederive_goals!(st, w)
        @test g.status === :declined
    end
    # alignment trigger surfaces a verified goal whose aggregate fell below target
    let st = M.State()
        g = M.Node(:g, "G-04"; title="rotten", status=:verified)
        g.attrs["fitness_kind"] = "count"
        g.fields[:fitness_target] = "2"
        st.nodes["G-04"] = g
        w = M.Node(:w, "W-04"; title="w", type=:feature, status=:done, cynefin=:clear)
        w.fields[:goals] = ["G-04"]
        w.fields[:fitness] = Dict("G-04" => 1)
        st.nodes["W-04"] = w
        M.record_id!(st, "G-04")
        M.record_id!(st, "W-04")
        trig = M.alignment_triggers(st)
        @test any(x -> occursin("verified goal below target: G-04", x) && occursin("total=1 target=2", x), trig)
    end
end
