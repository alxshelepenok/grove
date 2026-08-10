@testset "cli: packet pulls linked decisions assumptions questions and dor breakdown" begin
    idx = M.render_index(M.State())
    @test occursin("# Development dashboard", idx)
    pst = M.State()
    pst.nodes["G-PK"] = M.Node(:g, "G-PK"; title="gk", status=:verified)
    wpk = M.Node(:w, "W-PK"; title="Pkt", type=:feature, status=:ready, cynefin=:clear)
    wpk.fields[:goals] = ["G-PK"]
    wpk.fields[:ac] = ["criterion"]
    wpk.fields[:hypothesis] = ["h"]
    wpk.fields[:evidence_strategy] = ["how we know"]
    wpk.fields[:fitness] = Dict("G-PK" => 1)
    pst.nodes["W-PK"] = wpk
    pst.nodes["D-99"] = M.Node(:d, "D-99"; title="Dec", status=:accepted)
    b92 =
        M.Node(:b, "B-92"; title="Assum", status=:validated, cynefin=:clear); b92.fields[:vm] = ["v"]
    pst.nodes["B-92"] = b92
    q98 = M.Node(:q, "Q-98"; title="Why", status=:answered, cynefin=:clear)
    q98.fields[:outcome] = ["here"]
    pst.nodes["Q-98"] = q98
    push!(pst.edges, M.Edge("W-PK", :implements, "D-99"))
    push!(pst.edges, M.Edge("B-92", :targets, "W-PK"))
    push!(pst.edges, M.Edge("Q-98", :asks, "W-PK"))
    pkt_txt = M.packet(pst, wpk)
    @test occursin("Execution packet", pkt_txt)
    @test occursin("## Decision D-99", pkt_txt)
    @test occursin("## Assumption B-92", pkt_txt)
    @test occursin("## Question Q-98", pkt_txt)
    @test occursin("**outcome:**", pkt_txt)
    @test occursin("## Definition of Ready", pkt_txt)

    tr = mktempdir()
    try
        @test M.main(["init", "--root=$tr", "--quiet"]) == 0
        lk = joinpath(tr, ".grove", "state.lock")
        ln = split(read(lk, String), '\n')
        ln[3] = "# checksum: sha256:$(repeat('0', 64))"
        write(lk, join(ln, '\n'))
        @test M.main(["repair", "--confirm", "--root=$tr"]) == 0
        @test M.main(["check", "--root=$tr"]) == 0
    finally
        rm(tr; recursive=true, force=true)
    end

    ta = mktempdir()
    try
        @test M.main(["init", "--root=$ta", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$ta", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=ArchTest", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$ta", "--quiet"]) ==
              0
        @test M.main(["archive", "G-01", "--root=$ta"]) == M.EXIT_GUARD
        @test M.main(["set", "G-01", "status=verified", "--root=$ta", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$ta"]) == M.EXIT_GUARD
        @test M.main(["distill", "G-01", "--null", "--root=$ta", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$ta"]) == 0
        stA = M.read_lock(joinpath(ta, ".grove", "state.lock"); verify=true)
        @test stA.nodes["G-01"].archived
    finally
        rm(ta; recursive=true, force=true)
    end
end
