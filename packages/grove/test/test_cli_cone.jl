function cone_run_cli(args)
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
    rc[], txt
end

function cone_md_section(md, heading, following)
    i = findfirst(heading, md)
    i === nothing && return ""
    j = findfirst(following, md)
    j === nothing && return md[last(i):end]
    md[last(i):first(j)-1]
end

function cone_fixture(tmp)
    @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
    @test M.main(["add", "g", "--title=Reliable auth", "--fitness-kind=count", "--fitness-target=5", "--area=A-01",
                  "--root=$tmp", "--quiet"]) == 0
    for t in ("Auth schema", "Token store", "Login flow", "SSO login")
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=$t", "--root=$tmp", "--quiet"]) == 0
    end
    for (a, b) in (("W-01", "W-03"), ("W-02", "W-03"), ("W-03", "W-04"),
                   ("G-01", "W-01"), ("G-01", "W-02"))
        @test M.main(["link", a, "blocks", b, "--root=$tmp", "--quiet"]) == 0
    end
end

@testset "cli cone: contraction order section renders topological backward cone" begin
    tmp = mktempdir()
    try
        cone_fixture(tmp)
        rc, md = cone_run_cli(["packet", "W-04", "--cone", "--root=$tmp"])
        @test rc == 0
        @test occursin("## Contraction order", md)
        order = [m.captures[1] for m in eachmatch(r"(?m)^\d+\.\s+(\S+)", md)]
        @test order == ["G-01", "W-01", "W-02", "W-03"]
        @test occursin("## Forward cone (impact)", md)
        @test occursin("## Fragility", md)
        @test !occursin("## Relevant discoveries", md)
        @test !occursin("cone truncated", md)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli cone: forward cone lists downstream impact" begin
    tmp = mktempdir()
    try
        cone_fixture(tmp)
        rc, md = cone_run_cli(["packet", "W-01", "--cone", "--root=$tmp"])
        @test rc == 0
        fwd = cone_md_section(md, "## Forward cone (impact)", "## Fragility")
        @test occursin("- W-03  ", fwd)
        @test occursin("- W-04  ", fwd)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli cone: fragility counts vertex-disjoint goal paths" begin
    tmp = mktempdir()
    try
        cone_fixture(tmp)
        rc, md = cone_run_cli(["packet", "W-03", "--cone", "--root=$tmp"])
        @test rc == 0
        @test occursin("- G-01: 2 disjoint blocks-paths", md)
        rc, md = cone_run_cli(["packet", "W-04", "--cone", "--root=$tmp"])
        @test rc == 0
        @test occursin("- G-01: 1 (brittle)", md)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli cone: depth horizon truncates backward cone with note" begin
    tmp = mktempdir()
    try
        cone_fixture(tmp)
        rc, md = cone_run_cli(["packet", "W-04", "--cone", "--cone-depth=1", "--root=$tmp"])
        @test rc == 0
        order = [m.captures[1] for m in eachmatch(r"(?m)^\d+\.\s+(\S+)", md)]
        @test order == ["W-03"]
        @test occursin("> cone truncated (depth=1, max=50)", md)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli cone: json mode exposes structured cone object" begin
    tmp = mktempdir()
    try
        cone_fixture(tmp)
        rc, txt = cone_run_cli(["packet", "W-04", "--cone", "--json", "--root=$tmp"])
        @test rc == 0
        d = JSON.parse(txt)
        @test d["command"] == "packet"
        @test d["work"] == "W-04"
        @test haskey(d, "cone")
        cone = d["cone"]
        @test cone["backward"] == ["W-03", "W-01", "W-02", "G-01"]
        @test cone["order"] == ["G-01", "W-01", "W-02", "W-03"]
        @test cone["forward"] == []
        @test cone["fragility"] == [Dict{String,Any}("goal" => "G-01", "paths" => 1)]
        @test cone["relevant_discoveries"] == []
        @test cone["truncated"] == false
        @test cone["depth"] == 4
        @test cone["max"] == 50
        @test occursin("## Contraction order", d["packet_markdown"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "cli cone: isolated work item reports no blocks-path and exits zero" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=Solo", "--root=$tmp", "--quiet"]) == 0
        rc, md = cone_run_cli(["packet", "W-01", "--cone", "--root=$tmp"])
        @test rc == 0
        @test occursin("## Contraction order", md)
        @test occursin("- G-01: no blocks-path", md)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "algebra: node_connectivity degenerate cases return zero" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="w2", type=:feature, status=:proposed, cynefin=:clear)
    @test M.node_connectivity(st, "G-01", "G-01") == 0
    @test M.node_connectivity(st, "G-01", "W-99") == 0
    @test M.node_connectivity(st, "W-99", "G-01") == 0
    @test M.node_connectivity(st, "G-01", "W-01") == 0
    push!(st.edges, M.Edge("G-01", :blocks, "W-01"))
    push!(st.edges, M.Edge("W-01", :blocks, "W-02"))
    @test M.node_connectivity(st, "G-01", "W-02") == 1
end

@testset "algebra: relevant_discoveries empty without discoveries" begin
    st = M.State()
    w = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-01"] = w
    @test M.relevant_discoveries(st, w, ["W-01"]) == String[]
end

@testset "cli cone: relevant discoveries lists only active Discoveries and truncates to cone max" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        mkpath(joinpath(tmp, "src"))
        write(joinpath(tmp, "src", "a.jl"), "")
        open(joinpath(tmp, ".grove", "glossary.md"), "a") do io
            println(io, "| auth | a term | test |")
        end
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1", "--area=A-01",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01",
                      "--title=W", "--surface=src/a.jl", "--root=$tmp", "--quiet"]) == 0
        for i in 1:5
            @test M.main(["add", "y", "--title=A$i", "--tags=auth", "--surface=src/a.jl",
                          "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=stale", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-04", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-05", "status=active", "--root=$tmp", "--quiet"]) == 0

        rc, txt = cone_run_cli(["packet", "W-01", "--cone", "--json", "--root=$tmp"])
        @test rc == 0
        d = JSON.parse(txt)
        @test d["cone"]["relevant_discoveries"] == ["Y-01", "Y-04", "Y-05"]
        @test occursin("## Relevant discoveries", d["packet_markdown"])
        @test occursin("- Y-01  ", d["packet_markdown"])
        @test !occursin("Y-02", d["packet_markdown"])
        @test !occursin("Y-03", d["packet_markdown"])

        rc, txt = cone_run_cli(["packet", "W-01", "--cone", "--cone-max=2", "--json", "--root=$tmp"])
        @test rc == 0
        d = JSON.parse(txt)
        @test d["cone"]["relevant_discoveries"] == ["Y-01", "Y-04"]
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "algebra: node_connectivity excludes archived nodes" begin
    st = M.State()
    st.nodes["G-01"] = M.Node(:g, "G-01"; title="g", status=:unverified)
    st.nodes["W-01"] = M.Node(:w, "W-01"; title="w", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-02"] = M.Node(:w, "W-02"; title="w2", type=:feature, status=:proposed, cynefin=:clear)
    st.nodes["W-03"] = M.Node(:w, "W-03"; title="w3", type=:feature, status=:proposed, cynefin=:clear)
    push!(st.edges, M.Edge("G-01", :blocks, "W-02"))
    push!(st.edges, M.Edge("W-02", :blocks, "W-01"))
    push!(st.edges, M.Edge("G-01", :blocks, "W-03"))
    push!(st.edges, M.Edge("W-03", :blocks, "W-01"))
    @test M.node_connectivity(st, "G-01", "W-01") == 2
    st.nodes["W-02"].archived = true
    @test M.node_connectivity(st, "G-01", "W-01") == 1
    st.nodes["G-01"].archived = true
    @test M.node_connectivity(st, "G-01", "W-01") == 0
    @test M.node_connectivity(st, "W-01", "W-02") == 0
end

@testset "cli cone: packet rejects non-positive cone depth and max" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["packet", "W-01", "--cone", "--cone-depth=0", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["packet", "W-01", "--cone", "--cone-max=0", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["packet", "W-01", "--cone", "--cone-depth=-2", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["packet", "W-01", "--cone", "--cone-max=-2", "--root=$tmp", "--quiet"]) == 1
        @test M.main(["packet", "W-01", "--cone", "--cone-depth=2", "--cone-max=3",
                      "--root=$tmp", "--quiet"]) == 0
    finally
        rm(tmp; recursive=true, force=true)
    end
end
