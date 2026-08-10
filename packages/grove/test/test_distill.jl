function distill_drive_verified(tmp::AbstractString)
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
    st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
    @test st.nodes["G-01"].status === :verified
end

function distill_capture(args)
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

function distill_capture_err(args)
    err_path, err_io = mktemp()
    close(err_io)
    rc = -1
    open(err_path, "w") do f
        redirect_stderr(f) do
            rc = M.main(args)
        end
    end
    txt = read(err_path, String)
    rm(err_path; force=true)
    (rc, txt)
end

distill_jlines(jp::AbstractString) = [JSON.parse(l) for l in readlines(jp) if !isempty(strip(l))]

@testset "distill: worksheet lists validated B answered Q accepted D from goal mass" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        @test M.main(["add", "b", "--title=Bench", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "q", "--title=Quest", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "d", "--title=Dec", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "B-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "Q-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "B-01", "status=validated", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Q-01", "status=answered", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        rc, out = distill_capture(["distill", "G-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("distillation worksheet for G-01", out)
        @test occursin("archive precondition: not met", out)
        @test occursin("- B-01 (validated B): Bench", out)
        @test occursin("- Q-01 (answered Q): Quest", out)
        @test occursin("- D-01 (accepted D): Dec", out)
        @test occursin("grove add y --from=B-01", out)
        @test occursin("grove add y --from=Q-01", out)
        @test occursin("grove add y --from=D-01", out)
        @test occursin("grove distill G-01 --null", out)
        rcj, outj = distill_capture(["distill", "G-01", "--root=$tmp", "--json"])
        @test rcj == 0
        d = JSON.parse(outj)
        @test d["command"] == "distill"
        @test d["goal"] == "G-01"
        @test d["precondition_met"] == false
        ids = [c["id"] for c in d["candidates"]]
        @test "B-01" in ids && "Q-01" in ids && "D-01" in ids
        @test all(c -> occursin("grove add y --from=", c["skeleton"]), d["candidates"])
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: refuses unknown id non-goal and unverified goal" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "a", "--title=Area", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "g", "--title=G", "--fitness-kind=count", "--fitness-target=1", "--area=A-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "w", "--type=feature", "--cynefin=clear", "--title=W",
                      "--root=$tmp", "--quiet"]) == 0
        @test M.main(["distill", "G-99", "--root=$tmp", "--quiet"]) == M.EXIT_NOTFOUND
        @test M.main(["distill", "W-01", "--root=$tmp", "--quiet"]) != 0
        @test M.main(["distill", "G-01", "--root=$tmp", "--quiet"]) == M.EXIT_GUARD
        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == M.EXIT_GUARD
        jp = joinpath(tmp, ".grove", "journal.log")
        @test !any(r -> get(r, "cmd", "") == "distill", distill_jlines(jp))
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: --null attestation lands as non-mutation journal record, undo skips it" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        jp = joinpath(tmp, ".grove", "journal.log")
        nlines = length(distill_jlines(jp))
        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == 0
        recs = distill_jlines(jp)
        @test length(recs) == nlines + 1
        rec = recs[end]
        @test rec["cmd"] == "distill"
        @test rec["inv"]["op"] == "distill"
        @test rec["inv"]["goal"] == "G-01"
        @test rec["inv"]["empty"] == true
        @test haskey(rec, "ts") && !isempty(rec["ts"])
        @test !M.journal_record_mutation(rec)
        @test M.distill_null_attested(jp, "G-01")
        @test !M.distill_null_attested(jp, "G-02")
        @test M.main(["add", "q", "--title=qq", "--cynefin=clear", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["undo", "--root=$tmp", "--quiet"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !haskey(st.nodes, "Q-01")
        @test haskey(st.nodes, "W-01")
        recs2 = distill_jlines(jp)
        @test count(r -> get(r, "cmd", "") == "distill", recs2) == 1
        @test M.distill_null_attested(jp, "G-01")
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: archive refuses without distillation then opens after --null" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        rc, etxt = distill_capture_err(["archive", "G-01", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("archive: distill G-01 first (grove distill G-01, or grove distill G-01 --null)",
                       etxt)
        st0 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test !st0.nodes["G-01"].archived
        @test M.main(["distill", "G-01", "--null", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$tmp"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].archived
        @test st.nodes["W-01"].archived
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: archive opens with a Discovery linked via produces from mass work" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        @test M.main(["add", "y", "--title=Seam", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        rc, out = distill_capture(["distill", "G-01", "--root=$tmp"])
        @test rc == 0
        @test occursin("archive precondition: met", out)
        @test occursin("Y-01", out)
        @test M.main(["archive", "G-01", "--root=$tmp"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].archived
        @test st.nodes["W-01"].archived
        @test !st.nodes["Y-01"].archived
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: archive opens with a Discovery linked via distills into mass record" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        @test M.main(["add", "d", "--title=Dec", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["link", "W-01", "produces", "D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "D-01", "status=accepted", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["add", "y", "--title=Seam", "--tags=auth", "--surface=src/a.jl",
                      "--from=D-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        st0 = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test any(e -> e.label === :distills && e.from == "Y-01" && e.to == "D-01", st0.edges)
        @test M.main(["archive", "G-01", "--root=$tmp"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].archived
        @test st.nodes["D-01"].archived
        @test !st.nodes["Y-01"].archived
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: archive gate counts only active Discoveries" begin
    tmp = mktempdir()
    try
        distill_drive_verified(tmp)
        @test M.main(["add", "y", "--title=Stale", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        rc, etxt = distill_capture_err(["archive", "G-01", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("archive: distill G-01 first", etxt)
        @test M.main(["set", "Y-01", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-01", "status=stale", "--root=$tmp", "--quiet"]) == 0
        rc, etxt = distill_capture_err(["archive", "G-01", "--root=$tmp"])
        @test rc == M.EXIT_GUARD
        @test occursin("archive: distill G-01 first", etxt)
        @test M.main(["add", "y", "--title=Live", "--tags=auth", "--surface=src/a.jl",
                      "--from=W-01", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["set", "Y-02", "status=active", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["archive", "G-01", "--root=$tmp"]) == 0
        st = M.read_lock(joinpath(tmp, ".grove", "state.lock"))
        @test st.nodes["G-01"].archived
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: add r is rejected as unknown kind" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        rc, etxt = distill_capture_err(["add", "r", "--title=Retro", "--root=$tmp"])
        @test rc != 0
        @test occursin("unknown kind: r", etxt)
    finally
        rm(tmp; recursive=true, force=true)
    end
end

@testset "distill: set done prints distill hint on stderr for newly verified goal" begin
    tmp = mktempdir()
    try
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
        rc, etxt = distill_capture_err(["set", "W-01", "status=done", "--root=$tmp", "--quiet"])
        @test rc == 0
        @test occursin("grove: goal G-01", etxt)
        @test occursin("grove distill G-01", etxt)
    finally
        rm(tmp; recursive=true, force=true)
    end
end
