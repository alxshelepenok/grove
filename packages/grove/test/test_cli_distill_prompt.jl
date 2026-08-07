@testset "goal: notes distill deferred marker suppresses verification nag" begin
    g = M.Node(:g, "G-01"; title="t", status=:unverified)
    @test !M.goal_notes_distill_deferred(g)
    g.fields[:notes] = ["no marker"]
    @test !M.goal_notes_distill_deferred(g)
    g.fields[:notes] = ["plan --distill-deferred until Friday"]
    @test M.goal_notes_distill_deferred(g)
end

@testset "cli: verified goal prompts distillation unless deferred in notes" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=Track", "--fitness=1/1 milestones", "--area=A-01", "--root=$tmp", "--quiet"]) ==
              0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=Do",
                      "--root=$tmp", "--quiet"]) == 0
        for (fn, ln) in (("ac", "a"), ("hypothesis", "h"), ("evidence_strategy", "s"))
            @test M.main(["field", "W-01", fn, "add", ln, "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"); verify=false)
        w = st.nodes["W-01"]
        oldg = Dict{String,String}()
        for gid in get(w.fields, :goals, String[])
            g = get(st.nodes, gid, nothing)
            g === nothing || (oldg[gid] = string(g.status))
        end
        w.status = :done
        M.rederive_goals!(st, w)
        buf = IOBuffer()
        M.print_lazy_distill_prompt_on_newly_verified_goals!(buf, st, w, oldg)
        msg = String(take!(buf))
        @test occursin("grove: goal G-01", msg)
        @test occursin("grove distill G-01", msg)
        @test occursin("--null", msg)
        @test st.nodes["G-01"].status === :verified
    finally
        rm(tmp; recursive=true)
    end
end

@testset "cli: distill prompt suppressed when goal notes carry defer marker" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=T", "--fitness=1/1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["field", "G-01", "notes", "add", "distill --distill-deferred", "--root=$tmp",
                      "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--goals=G-01", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        for (fn, ln) in (("ac", "a"), ("hypothesis", "h"), ("evidence_strategy", "s"))
            @test M.main(["field", "W-01", fn, "add", ln, "--root=$tmp", "--quiet"]) == 0
        end
        @test M.main(["fitness", "W-01", "G-01", "1", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["evidence", "W-01", "e", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"); verify=false)
        w = st.nodes["W-01"]
        oldg = Dict("G-01" => string(st.nodes["G-01"].status))
        w.status = :done
        M.rederive_goals!(st, w)
        buf = IOBuffer()
        M.print_lazy_distill_prompt_on_newly_verified_goals!(buf, st, w, oldg)
        msg = String(take!(buf))
        @test isempty(strip(msg))
    finally
        rm(tmp; recursive=true)
    end
end
