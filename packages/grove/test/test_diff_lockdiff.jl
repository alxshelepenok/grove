@testset "cli: init + add + set + check" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test isfile(joinpath(tmp, ".grove", "state.lock"))
    @test isfile(joinpath(tmp, ".grove", "index.md"))
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=Migrate", "--fitness=5/5 modules", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=Add login", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["check", "--root=$tmp", "--quiet"]) == 0
    @test !isdir(joinpath(tmp, ".grove", "locks", "exclusive"))
    rm(tmp; recursive=true)
end

@testset "cli --json read shapes (stdout)" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=X", "--fitness=1/1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        for fn = ("ac", "hypothesis", "evidence_strategy")
            @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
        function run_json_cmd(args)
            out_path, out_io = mktemp()
            close(out_io)
            rc = Ref(-1)
            open(out_path, "w") do f
                redirect_stdout(f) do
                    rc[] = M.main(args)
                end
            end
            txt = read(out_path, String)
            rm(out_path, force=true)
            rc[], JSON.parse(txt)
        end
        r, d = run_json_cmd(["check", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "check"
        @test d["ok"] == true
        r, d = run_json_cmd(["dor", "W-01", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "dor"
        @test d["work"] == "W-01"
        @test d["dor"] isa Bool
        @test length(d["conjuncts"]) >= 3
        r, d = run_json_cmd(["status", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "status"
        @test haskey(d, "progress") && haskey(d, "invariants")
        r, d = run_json_cmd(["deps", "W-01", "--root=$tmp", "--json"])
        @test r == 0
        @test d["command"] == "deps"
        @test d["predecessors"] isa AbstractVector
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "alignment_triggers: chaotic Q" begin
    st = M.State()
    st.nodes["Q-01"] = M.Node(:q, "Q-01"; title="q", status=:open, cynefin=:chaotic)
    t = M.alignment_triggers(st)
    @test any(x -> occursin("chaotic", x), t)
end

@testset "cli: status" begin
    tmp = mktempdir()
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["status", "--root=$tmp", "--quiet"]) == 0
    rm(tmp; recursive=true)
end

@testset "lockdiff: ignores prose bullet order" begin
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

@testset "lockdiff: ignores edge declaration order" begin
    st = M.State()
    ga = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["G-01"] = ga
    M.record_id!(st, "G-01")
    wa = M.Node(:w, "W-01"; title="w1", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-01"] = wa
    M.record_id!(st, "W-01")
    wb = M.Node(:w, "W-02"; title="w2", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-02"] = wb
    M.record_id!(st, "W-02")
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("G-01", :blocks, "W-01"))
    sto = Base.deepcopy(st)
    reverse!(st.edges)
    @test sto.edges != st.edges
    report = join(M.lock_structural_lines(sto, st), '\n')
    @test occursin("(no semantic changes)", report)
end

@testset "lockdiff: title change surfaced" begin
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

@testset "lockdiff: JSON payload no change for deepcopy" begin
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

@testset "cli: diff in git sandbox" begin
    hasgit = success(pipeline(`git --version`; stdout=devnull, stderr=devnull))
    hasgit || return
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test success(pipeline(`git -C $tmp init`; stdout=devnull, stderr=devnull))
        @test success(pipeline(`git -C $tmp config user.email diff@test.local`; stderr=devnull))
        @test success(pipeline(`git -C $tmp config user.name "grove tests"`; stderr=devnull))
        @test success(pipeline(`git -C $tmp add .`; stderr=devnull))
        @test success(pipeline(`git -C $tmp commit -m init`; stderr=devnull))
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear",
                      "--title=PostCommit", "--root=$tmp", "--quiet"]) == 0
        cap = joinpath(tmp, "_cap.txt")
        rc = open(cap, "w") do fh
            redirect_stdout(fh) do
                M.main(["diff", "--since=HEAD", "--root=$tmp"])
            end
        end
        @test rc == 0
        s = read(cap, String)
        @test occursin("grove diff", s)
        @test occursin("### added (+)", s)
    finally
        rm(tmp; recursive=true)
    end
end

@testset "cli: diff without git exits with error" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["diff", "--root=$tmp"]) == 1
    finally
        rm(tmp; recursive=true)
    end
end
