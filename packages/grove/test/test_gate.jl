function gate_test_drive_done(tmp::AbstractString)
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                  "--title=W", "--root=$tmp", "--quiet"]) == 0
    for fn in ("ac", "hypothesis", "evidence_strategy")
        @test M.main(["field", "W-01", fn, "add", "x", "--root=$tmp", "--quiet"]) == 0
    end
    @test M.main(["fitness", "W-01", "G-01", "+1", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-01", "status=ready", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-01", "status=progress", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["set", "W-01", "status=done", "--root=$tmp", "--quiet"]) == 0
end

function gate_test_capture(args)
    out_path, out_io = mktemp()
    close(out_io)
    rc = -1
    open(out_path, "w") do f
        redirect_stdout(f) do
            rc = M.main(args)
        end
    end
    txt = read(out_path, String)
    rm(out_path; force=true)
    (rc, txt)
end

gate_test_jlines(jp::AbstractString) = [JSON.parse(l) for l in readlines(jp) if !isempty(strip(l))]

@testset "treewidth: min-fill bound on empty tree cycle clique and archived" begin
    @test M.treewidth_upper(M.State()) == 0
    lone = M.State()
    lone.nodes["W-01"] = M.Node(:w, "W-01"; title="w")
    @test M.treewidth_upper(lone) == 0
    tree = M.State()
    for id in ("W-01", "W-02", "W-03", "W-04")
        tree.nodes[id] = M.Node(:w, id; title=id)
    end
    push!(tree.edges, M.Edge("W-01", :blocks, "W-02"))
    push!(tree.edges, M.Edge("W-02", :blocks, "W-03"))
    push!(tree.edges, M.Edge("W-03", :blocks, "W-04"))
    @test M.treewidth_upper(tree) == 1
    cyc = M.State()
    for id in ("W-01", "W-02", "W-03", "W-04")
        cyc.nodes[id] = M.Node(:w, id; title=id)
    end
    for (a, b) in (("W-01", "W-02"), ("W-02", "W-03"), ("W-03", "W-04"), ("W-04", "W-01"))
        push!(cyc.edges, M.Edge(a, :blocks, b))
    end
    @test M.treewidth_upper(cyc) >= 2
    k4 = M.State()
    ids = ["W-01", "W-02", "W-03", "W-04"]
    for id in ids
        k4.nodes[id] = M.Node(:w, id; title=id)
    end
    for i in 1:4, j in i+1:4
        push!(k4.edges, M.Edge(ids[i], :blocks, ids[j]))
    end
    @test M.treewidth_upper(k4) == 3
    k4.nodes["W-04"].archived = true
    @test M.treewidth_upper(k4) == 2
end

@testset "cli: gate first run baseline none then baseline and null record" begin
    tmp = mktempdir()
    try
        gate_test_drive_done(tmp)
        jp = joinpath(tmp, ".grove", "journal.log")
        lk = joinpath(tmp, ".grove", "state.lock")
        lock_before = read(lk, String)
        nlines = length(readlines(jp))
        rc1, out1 = gate_test_capture(["gate", "--root=$tmp", "--n=1"])
        @test rc1 == 0
        @test occursin("baseline: none", out1)
        @test occursin("done since baseline: 1", out1)
        @test occursin("due: true", out1)
        @test read(lk, String) == lock_before
        @test length(readlines(jp)) == nlines + 1
        rec1 = gate_test_jlines(jp)[end]
        @test rec1["cmd"] == "gate"
        @test rec1["inv"]["op"] == "gate"
        @test rec1["inv"]["dones"] == 1
        @test haskey(rec1["inv"], "tw") && haskey(rec1["inv"], "empty")
        rc2, out2 = gate_test_capture(["gate", "--root=$tmp"])
        @test rc2 == 0
        @test !occursin("baseline: none", out2)
        @test occursin("baseline: ", out2)
        @test occursin("treewidth: ", out2)
        @test occursin("due: false", out2)
        @test length(readlines(jp)) == nlines + 2
        rec2 = gate_test_jlines(jp)[end]
        @test rec2["inv"]["op"] == "gate"
        @test rec2["inv"]["empty"] == true
        @test read(lk, String) == lock_before
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: gate lists invalidated B and accepted D as distill candidates" begin
    tmp = mktempdir()
    try
        gate_test_drive_done(tmp)
        @test M.main(["add", "b", "--title=fragile", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "B-01", "status=invalidated_acceptable", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=use-jwt", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        rc, out = gate_test_capture(["gate", "--root=$tmp"])
        @test rc == 0
        @test occursin("- invalidated B-01: fragile", out)
        @test occursin("- accepted D-01: use-jwt", out)
        rcj, outj = gate_test_capture(["gate", "--root=$tmp", "--json"])
        @test rcj == 0
        d = JSON.parse(outj)
        @test d["command"] == "gate"
        @test haskey(d, "tw_now") && haskey(d, "tw_delta") && haskey(d, "dones")
        @test haskey(d, "due") && haskey(d, "overflows") && haskey(d, "invalidated")
        @test haskey(d, "accepted") && haskey(d, "baseline") && haskey(d, "empty")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: gate respects baseline time cut from journal" begin
    tmp = mktempdir()
    try
        gate_test_drive_done(tmp)
        jp = joinpath(tmp, ".grove", "journal.log")
        open(jp, "a") do io
            println(io, JSON.json(Dict{String,Any}(
                "v" => 1, "ts" => "2999-01-01T00:00:00Z", "cmd" => "gate",
                "inv" => Dict{String,Any}("op" => "gate", "tw" => 0, "dones" => 0, "empty" => true))))
        end
        rc1, out1 = gate_test_capture(["gate", "--root=$tmp"])
        @test rc1 == 0
        @test occursin("done since baseline: 0", out1)
        @test occursin("would distill: none", out1)
        open(jp, "a") do io
            println(io, JSON.json(Dict{String,Any}(
                "v" => 1, "ts" => "2000-01-01T00:00:00Z", "cmd" => "gate",
                "inv" => Dict{String,Any}("op" => "gate", "tw" => 0, "dones" => 0, "empty" => true))))
        end
        rc2, out2 = gate_test_capture(["gate", "--root=$tmp"])
        @test rc2 == 0
        @test occursin("done since baseline: 1", out2)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli: undo skips gate records and still reverts last mutation" begin
    tmp = mktempdir()
    try
        gate_test_drive_done(tmp)
        jp = joinpath(tmp, ".grove", "journal.log")
        rc0, _ = gate_test_capture(["gate", "--root=$tmp"])
        @test rc0 == 0
        @test M.main(["add", "q", "--title=qq", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        rc1, _ = gate_test_capture(["gate", "--root=$tmp"])
        @test rc1 == 0
        ngate = count(r -> get(r, "cmd", "") == "gate", gate_test_jlines(jp))
        @test ngate == 2
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st.nodes, "Q-01")
        @test haskey(st.nodes, "W-01")
        @test count(r -> get(r, "cmd", "") == "gate", gate_test_jlines(jp)) == 2
        rc2, outl = gate_test_capture(["log", "--root=$tmp", "--limit=500"])
        @test rc2 == 0
        @test occursin("\tjournal\tgate", outl)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "session: gate classified mutate xor read" begin
    @test "gate" in M.SESSION_MUTATE_COMMANDS
    @test !("gate" in M.SESSION_READ_COMMANDS)
    @test xor("gate" in M.SESSION_READ_COMMANDS, "gate" in M.SESSION_MUTATE_COMMANDS)
end

if Sys.which("git") !== nothing
    @testset "cli: gate surface overflow honors declared surface and theta" begin
        tmp = mktempdir()
        try
            run(`git -C $tmp init -q`)
            write(joinpath(tmp, "a.txt"), "a\n")
            run(`git -C $tmp add a.txt`)
            run(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m scaffold`)
            gate_test_drive_done(tmp)
            @test M.main(["field", "W-01", "surface", "add", "a.txt", "--root=$tmp", "--quiet"]) == 0
            write(joinpath(tmp, "a.txt"), "a2\n")
            write(joinpath(tmp, "b.txt"), "b\n")
            run(`git -C $tmp add a.txt b.txt`)
            run(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m "touch b for W-01"`)
            jp = joinpath(tmp, ".grove", "journal.log")
            rc1, out1 = gate_test_capture(["gate", "--root=$tmp", "--theta=1"])
            @test rc1 == 0
            @test !occursin("- overflow W-01", out1)
            rm(jp)
            rc0, out0 = gate_test_capture(["gate", "--root=$tmp", "--theta=0"])
            @test rc0 == 0
            @test occursin("- overflow W-01: b.txt", out0)
            @test !occursin("a.txt", out0)
        finally
            rm(tmp; recursive=true, force=true)
        end
    end

    @testset "cli: gate without git matches keeps overflow empty" begin
        tmp = mktempdir()
        try
            gate_test_drive_done(tmp)
            rc, out = gate_test_capture(["gate", "--root=$tmp", "--theta=0"])
            @test rc == 0
            @test !occursin("- overflow", out)
        finally
            rm(tmp; recursive=true, force=true)
        end
    end

    @testset "gate: batched git attributes W-01 and W-010 separately" begin
        tmp = mktempdir()
        try
            run(`git -C $tmp init -q`)
            write(joinpath(tmp, "one.txt"), "1\n")
            run(`git -C $tmp add one.txt`)
            run(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m "deliver W-01"`)
            write(joinpath(tmp, "ten.txt"), "10\n")
            run(`git -C $tmp add ten.txt`)
            run(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m "deliver W-010"`)
            st = M.State()
            st.nodes["W-01"] = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done)
            st.nodes["W-010"] = M.Node(:w, "W-010"; title="w10", type=:feature, status=:done)
            ov = Dict(M.surface_overflows(st, tmp, nothing))
            @test ov["W-01"] == ["one.txt"]
            @test ov["W-010"] == ["ten.txt"]
            st.nodes["W-01"].fields[:surface] = ["one.txt"]
            ov2 = Dict(M.surface_overflows(st, tmp, nothing))
            @test !haskey(ov2, "W-01")
            @test ov2["W-010"] == ["ten.txt"]
        finally
            rm(tmp; recursive=true, force=true)
        end
    end

    @testset "gate: batched git honors baseline since cut" begin
        tmp = mktempdir()
        try
            run(`git -C $tmp init -q`)
            write(joinpath(tmp, "old.txt"), "o\n")
            run(`git -C $tmp add old.txt`)
            run(setenv(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m "old W-01"`,
                "GIT_AUTHOR_DATE" => "2000-06-01T00:00:00Z",
                "GIT_COMMITTER_DATE" => "2000-06-01T00:00:00Z"))
            write(joinpath(tmp, "new.txt"), "n\n")
            run(`git -C $tmp add new.txt`)
            run(`git -C $tmp -c user.email=t@example.com -c user.name=t commit -q -m "new W-01"`)
            st = M.State()
            w = M.Node(:w, "W-01"; title="w1", type=:feature, status=:done)
            w.attrs["t_updated"] = "2026-01-01T00:00:00Z"
            st.nodes["W-01"] = w
            ov = Dict(M.surface_overflows(st, tmp, (ts="2020-01-01T00:00:00Z", tw=0, dones=0)))
            @test ov["W-01"] == ["new.txt"]
            ov0 = Dict(M.surface_overflows(st, tmp, nothing))
            @test ov0["W-01"] == ["new.txt", "old.txt"]
        finally
            rm(tmp; recursive=true, force=true)
        end
    end
end
