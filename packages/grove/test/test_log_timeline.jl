@testset "log: timeline sorts newest node stamp first unlimited when limit zero" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="G", status=:unverified)
    g.attrs["t_created"] = "2020-01-01T00:00:00Z"
    g.attrs["t_updated"] = "2021-06-01T00:00:00Z"
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    rows = M.log_timeline(st; limit=0)
    @test rows[1].ts == "2021-06-01T00:00:00Z"
    @test any(r -> occursin("\tupdated\t", r.line), rows)
end

@testset "log: timeline includes edge rows with timestamps" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="x", status=:unverified)
    g.attrs["t_created"] = "2026-01-01T00:00:00Z"
    g.attrs["t_updated"] = g.attrs["t_created"]
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="y", type=:feature, status=:proposed, cynefin=:clear)
    w.attrs["t_created"] = "2026-01-02T00:00:00Z"
    w.attrs["t_updated"] = w.attrs["t_created"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    eg = M.Edge("G-01", :blocks, "W-01")
    eg.t_created = "2026-03-03T12:00:00Z"
    push!(st.edges, eg)
    rows = M.log_timeline(st; limit=10)
    @test any(r -> occursin(r"^\d{4}-\d{2}-\d{2}.*\tedge\t", r.line), rows)
end

@testset "log: timeline id filter respects limit cap" begin
    st = M.State()
    g = M.Node(:g, "G-01"; title="only", status=:unverified)
    g.attrs["t_created"] = "2020-01-01T00:00:00Z"
    g.attrs["t_updated"] = "2020-03-03T00:00:00Z"
    st.nodes["G-01"] = g
    M.record_id!(st, "G-01")
    w = M.Node(:w, "W-01"; title="other", type=:feature, status=:proposed, cynefin=:clear)
    w.attrs["t_created"] = "2025-05-05T00:00:00Z"
    w.attrs["t_updated"] = w.attrs["t_created"]
    st.nodes["W-01"] = w
    M.record_id!(st, "W-01")
    r = M.log_timeline(st; idfilt="G-01", limit=1)
    @test length(r) == 1
    @test occursin("G-01", r[1].line)
end

@testset "log: timeline merges detached journal records by timestamp" begin
    st = M.State()
    d = mktempdir()
    jp = joinpath(d, "x.log")
    try
        invd = Dict("op" => "rm_node", "id" => "G-99")
        rec = Dict("v" => 1, "ts" => "2031-01-01T00:00:00Z", "cmd" => "add", "inv" => invd)
        open(jp, "w") do io
            println(io, JSON.json(rec))
        end
        rows = M.log_timeline(st; journal_path = jp, limit = 20)
        @test any(r -> occursin("\tjournal\tadd\trm_node id=G-99", r.line), rows)
        r2 = M.log_timeline(st; idfilt = "G-99", journal_path = jp, limit = 20)
        @test length(r2) >= 1
    finally
        rm(d; recursive = true)
    end
end

@testset "log: journal grep detects ids inside serialized entries" begin
    d = mktempdir()
    jp = joinpath(d, "j.log")
    try
        rec = Dict(
            "v" => 1,
            "ts" => "2032-01-01T00:00:00Z",
            "cmd" => "add",
            "inv" => Dict("op" => "rm_node", "id" => "G-77"),
        )
        open(jp, "w") do io
            println(io, JSON.json(rec))
        end
        @test M.journal_file_mentions_id(jp, "G-77")
        @test !M.journal_file_mentions_id(jp, "G-99")
    finally
        rm(d; recursive = true)
    end
end
