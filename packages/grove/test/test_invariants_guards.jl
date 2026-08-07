@testset "invariants: structural guards for evidence wip blocks fitness session and bchain" begin
    st_bad = M.State()
    st_bad.nodes["G-09"] = M.Node(:g, "G-09"; title="", status=:verified)
    wd = M.Node(:w, "W-D"; title="", type=:feature, status=:done, cynefin=:clear)
    wd.fields[:goals] = ["G-09"]; wd.fields[:fitness] = Dict("G-09" => 1)
    st_bad.nodes["W-D"] = wd
    @test length(M.i3_done_has_evidence(st_bad)) == 1
    wd.fields[:evidence] = ["x"]
    @test isempty(M.i3_done_has_evidence(st_bad))

    wip_st = M.State()
    for (i, id) in enumerate(("W-W1", "W-W2", "W-W3"))
        wip_st.nodes[id] =
            M.Node(:w, id; title="", type=:feature, status=:progress, cynefin=:clear)
        r = M.i4_wip_limit(wip_st)
        if i < 3
            @test isempty(r)
        else
            @test length(r) == 1 && occursin("I4:", r[1])
        end
    end
    empty_st = M.State()
    @test isempty(M.i4_wip_limit(empty_st))

    i5_bad = M.State()
    gv = M.Node(:g, "G-V"; title="", status=:unverified); i5_bad.nodes["G-V"] = gv
    wx = M.Node(:w, "W-P"; title="", type=:feature, status=:progress, cynefin=:clear)
    i5_bad.nodes["W-P"] = wx
    push!(i5_bad.edges, M.Edge("G-V", :blocks, "W-P"))
    @test any(x -> occursin("I5:", x), M.i5_blocks_terminal(i5_bad))
    gv.status = :verified
    @test isempty(M.i5_blocks_terminal(i5_bad))

    i10_bad = M.State()
    w10 = M.Node(:w, "W-10"; title="", type=:feature, status=:done, cynefin=:clear)
    w10.fields[:goals] = ["G-Q"]; w10.fields[:fitness] = Dict{String,Int}()
    i10_bad.nodes["G-Q"] = M.Node(:g, "G-Q"; title="", status=:verified)
    i10_bad.nodes["W-10"] = w10
    @test occursin("I10:", M.i10_done_fitness(i10_bad)[1])
    w10.fields[:fitness] = Dict("G-Q" => 1)
    @test isempty(M.i10_done_fitness(i10_bad))

    i11_bad = M.State()
    w11 = M.Node(:w, "W-I11"; title="", type=:feature, status=:progress, cynefin=:clear)
    i11_bad.nodes["W-I11"] = w11
    @test any(x -> occursin("I11:", x), M.i11_progress_has_session_claim(i11_bad))
    w11.attrs["session"] = "tok"; w11.attrs["session_at"] = "2099-01-01T00:00:00Z"
    @test isempty(M.i11_progress_has_session_claim(i11_bad))

    i9_bad = M.State()
    i9_bad.nodes["G-R"] = M.Node(:g, "G-R"; title="", status=:verified)
    wf9 = M.Node(:w, "W-I9"; title="", type=:feature, status=:ready, cynefin=:clear)
    wf9.fields[:goals] = ["G-R"]
    i9_bad.nodes["W-I9"] = wf9
    i9_bad.nodes["B-77"] =
        M.Node(:b, "B-77"; title="", status = :proposed, cynefin = :clear)
    push!(i9_bad.edges, M.Edge("B-77", :targets, "W-I9"))
    @test occursin("I9:", M.i9_feature_bchain(i9_bad)[1])
    i9_bad.nodes["B-77"].status = :validated
    @test isempty(M.i9_feature_bchain(i9_bad))
end

@testset "invariants: blocks graph must stay acyclic" begin
    st = M.State()
    for id in ("W-01", "W-02", "W-03")
        st.nodes[id] = M.Node(:w, id; title=id, type=:feature, status=:ready, cynefin=:clear)
    end
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("W-02", :blocks, "W-03"))
    @test isempty(M.i7_blocks_dag(st))
    push!(st.edges, M.Edge("W-03", :blocks, "W-01"))
    @test !isempty(M.i7_blocks_dag(st))
end
