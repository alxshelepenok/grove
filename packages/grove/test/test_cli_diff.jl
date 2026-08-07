@testset "cli: diff against HEAD succeeds inside git sandbox" begin
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

@testset "cli: diff without git repository exits non-zero" begin
    tmp = mktempdir()
    try
        @test M.main(["init", "--root=$tmp", "--quiet"]) == 0
        @test M.main(["diff", "--root=$tmp"]) == 1
    finally
        rm(tmp; recursive=true)
    end
end
