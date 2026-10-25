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

@testset "archive: shared resources retire with the last archived goal" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    g1.archived = true
    g2 = M.Node(:g, "G-02"; title="b", status=:verified)
    w1 = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01", "G-02"]
    st.nodes["G-01"] = g1
    st.nodes["G-02"] = g2
    st.nodes["W-01"] = w1
    M.record_id!(st, "G-01")
    M.record_id!(st, "G-02")
    M.record_id!(st, "W-01")
    orphans = M.archive_orphan_ids(st, "G-01")
    @test isempty(orphans)
    orphans = M.archive_orphan_ids(st, "G-02")
    @test orphans == Set(["W-01"])
end

@testset "archive: orphan detection covers transitive d/q/b and skips goal-less nodes and themes" begin
    st = M.State()
    g1 = M.Node(:g, "G-01"; title="a", status=:verified)
    g1.archived = true
    g2 = M.Node(:g, "G-02"; title="b", status=:verified)
    st.nodes["G-01"] = g1
    st.nodes["G-02"] = g2
    w1 = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done, cynefin=:clear)
    w1.fields[:goals] = ["G-01", "G-02"]
    st.nodes["W-01"] = w1
    d1 = M.Node(:d, "D-01"; title="d", status=:accepted)
    st.nodes["D-01"] = d1
    push!(st.edges, M.Edge("W-01", :implements, "D-01"))
    t1 = M.Node(:t, "T-01"; title="t", status=:open)
    st.nodes["T-01"] = t1
    w1.fields[:theme] = "T-01"
    orphan = M.Node(:w, "W-02"; title="goal-less", type=:bug, status=:proposed, cynefin=:clear)
    st.nodes["W-02"] = orphan
    M.record_id!(st, "G-01")
    M.record_id!(st, "G-02")
    M.record_id!(st, "W-01")
    M.record_id!(st, "D-01")
    M.record_id!(st, "T-01")
    M.record_id!(st, "W-02")
    orphans = M.archive_orphan_ids(st, "G-02")
    @test orphans == Set(["W-01", "D-01"])
    @test !("T-01" in orphans) && !("W-02" in orphans)
end

@testset "archive: cascade keeps goals fields intact while orphaned shared work retires with the last goal" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G1", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G2", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01,G-02", "--title=Shared", "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["fitness", "W-01", "G-02", "+1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].archived == false
        @test st.nodes["W-01"].fields[:goals] == ["G-01", "G-02"]
        @test M.main(["distill", "G-02", "--null", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-02", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["W-01"].archived == true
        @test st.nodes["W-01"].fields[:goals] == ["G-01", "G-02"]
        jp = joinpath(tmp, ".grove", "journal.log")
        _, recs = M.journal_read_nonempty_pairs(jp)
        @test recs[end]["inv"]["ids"] == ["G-02", "W-01"]
        @test M.main(["check", "--root=$tmp", "--quiet"]) == 0
    finally
        rm(tmp; recursive=true, force=true)
    end
end
