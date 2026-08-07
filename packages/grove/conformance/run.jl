if abspath(PROGRAM_FILE) == @__FILE__
    import Pkg
    Pkg.activate(normpath(joinpath(@__DIR__, "..")); io=devnull)
end

module Conformance

using grove
using JSON
using Dates

const G = grove

include("scenarios.jl")

const CORPUS_DIR = joinpath(@__DIR__, "corpus")

function capture_cmd(f)
    opath, oio = mktemp()
    close(oio)
    epath, eio = mktemp()
    close(eio)
    rc = -1
    open(opath, "w") do of
        open(epath, "w") do ef
            redirect_stdout(of) do
                redirect_stderr(ef) do
                    rc = f()
                end
            end
        end
    end
    out = read(opath, String)
    err = read(epath, String)
    rm(opath; force=true)
    rm(epath; force=true)
    rc, out, err
end

function normalize_text(s::AbstractString, paths, tokens)::String
    s = replace(s, "\r\n" => "\n")
    s = replace(s, "\r" => "\n")
    for (p, ph) in paths
        s = replace(s, replace(p, '\\' => "\\\\") => ph)
        s = replace(s, p => ph)
        fwd = replace(p, '\\' => '/')
        fwd == p || (s = replace(s, fwd => ph))
    end
    for (p, ph) in paths
        esc = replace(ph, r"([.^$*+?()\[\]{}|])" => s"\\\1")
        pat = Regex(esc * raw"[A-Za-z0-9_.:/\\-]+")
        s = replace(s, pat => m -> replace(m, "\\\\" => "/", "\\" => "/"))
    end
    s = replace(s, r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z" => "<ts>")
    s = replace(s, r"sha256:[0-9a-f]{64}" => "sha256:<sha>")
    s = replace(s, r"(?<![0-9a-fA-F])[0-9a-f]{64}(?![0-9a-fA-F])" => "<sha>")
    for t in tokens
        s = replace(s, t => "<session>")
        length(t) > 24 && (s = replace(s, first(t, 24) => "<session>"))
    end
    replace(s, r"[ \t]+$"m => "")
end

function retime_journal(target::AbstractString)::Int
    jp = joinpath(target, ".grove", "journal.log")
    isfile(jp) || error("retime-journal: no journal at $jp")
    lines = split(read(jp, String), '\n'; keepempty=false)
    base = Dates.DateTime(2026, 1, 1)
    io = IOBuffer()
    for (i, ln) in enumerate(lines)
        ts = Dates.format(base + Dates.Hour(i - 1), "yyyy-mm-ddTHH:MM:SSZ")
        println(io, replace(ln, r"\"ts\":\"[^\"]*\"" => "\"ts\":\"$ts\""; count=1))
    end
    write(jp, String(take!(io)))
    0
end

function run_pseudo(step::Vector{String}, target::AbstractString)
    op = step[1]
    if op == "!write"
        length(step) >= 2 || error("!write needs a path")
        p = joinpath(target, step[2])
        mkpath(dirname(p))
        write(p, join(step[3:end], " "))
        return 0, "", ""
    elseif op == "!append"
        length(step) >= 2 || error("!append needs a path")
        p = joinpath(target, step[2])
        mkpath(dirname(p))
        open(p, "a") do io
            println(io, join(step[3:end], " "))
        end
        return 0, "", ""
    elseif op == "!rm"
        length(step) >= 2 || error("!rm needs a path")
        rm(joinpath(target, step[2]); force=true)
        return 0, "", ""
    elseif op == "!cat"
        length(step) >= 2 || error("!cat needs a path")
        p = joinpath(target, step[2])
        isfile(p) || error("!cat: no file at $p")
        return 0, read(p, String), ""
    elseif op == "!git"
        so = IOBuffer()
        se = IOBuffer()
        gargs = step[2:end]
        proc = run(pipeline(`git -C $target $gargs`; stdout=so, stderr=se); wait=false)
        wait(proc)
        proc.exitcode == 0 ||
            error("git step failed ($(proc.exitcode)): $(join(step, " "))\n$(String(take!(se)))")
        return 0, "", ""
    elseif op == "!retime-journal"
        return retime_journal(target), "", ""
    elseif op == "!sleep"
        length(step) >= 2 || error("!sleep needs seconds")
        sleep(parse(Float64, step[2]))
        return 0, "", ""
    end
    error("unknown pseudo-step: $op")
end

function exec_step(sc, i::Int, raw::Vector{String}, root::AbstractString,
                   root2::AbstractString, norm)
    step = copy(raw)
    target = root
    if !isempty(step) && step[1] == "@2"
        sc.two_roots || error("scenario $(sc.name) step $i uses @2 without two_roots")
        target = root2
        popfirst!(step)
    end
    isempty(step) && error("scenario $(sc.name) step $i is empty")
    sub(s) = replace(replace(s, "{root2}" => root2), "{root}" => root)
    if startswith(step[1], "!")
        rc, out, err = run_pseudo([sub(a) for a in step], target)
    else
        args = [sub(a) for a in step]
        rc, out, err = capture_cmd(() -> G.main(vcat(args, ["--root=$target"])))
    end
    lockp = joinpath(target, ".grove", "state.lock")
    jp = joinpath(target, ".grove", "journal.log")
    locktxt = isfile(lockp) ? norm(read(lockp, String)) : nothing
    jtxt = isfile(jp) ? norm(read(jp, String)) : nothing
    Pair{String,Any}[
        "args" => raw,
        "exit" => rc,
        "stdout" => norm(out),
        "stderr" => norm(err),
        "lock" => locktxt,
        "journal" => jtxt,
    ]
end

function run_scenario(sc)
    base = mktempdir()
    home = mktempdir()
    root = joinpath(base, "main")
    mkpath(root)
    root2 = joinpath(base, "other")
    sc.two_roots && mkpath(root2)
    keys = ("GROVE_HOME", "GROVE_PROJECT", "GROVE_SESSION")
    saved = Dict(k => get(ENV, k, nothing) for k in keys)
    recs = Any[]
    pending = nothing
    try
        ENV["GROVE_HOME"] = home
        delete!(ENV, "GROVE_PROJECT")
        delete!(ENV, "GROVE_SESSION")
        paths = Tuple{String,String}[(root, "<root>"), (home, "<home>")]
        sc.two_roots && push!(paths, (root2, "<root2>"))
        tokens = [G.derive_default_session_token(root)]
        sc.two_roots && push!(tokens, G.derive_default_session_token(root2))
        norm(s) = normalize_text(s, paths, tokens)
        for (i, raw) in enumerate(sc.steps)
            push!(recs, exec_step(sc, i, raw, root, root2, norm))
        end
    catch e
        pending = e
    finally
        for k in keys
            saved[k] === nothing ? delete!(ENV, k) : (ENV[k] = saved[k])
        end
        try
            rm(base; recursive=true, force=true)
        catch
        end
        try
            rm(home; recursive=true, force=true)
        catch
        end
    end
    pending === nothing || rethrow(pending)
    recs
end

function json_write(io::IO, x, ind::Int)::Nothing
    pad = "  "^ind
    pad2 = "  "^(ind + 1)
    if x isa AbstractVector && !isempty(x) && first(x) isa Pair
        print(io, "{\n")
        for (i, (k, v)) in enumerate(x)
            print(io, pad2, JSON.json(string(k)), ": ")
            json_write(io, v, ind + 1)
            i < length(x) && print(io, ",")
            print(io, "\n")
        end
        print(io, pad, "}")
    elseif x isa AbstractVector
        if isempty(x)
            print(io, "[]")
        else
            print(io, "[\n")
            for (i, v) in enumerate(x)
                print(io, pad2)
                json_write(io, v, ind + 1)
                i < length(x) && print(io, ",")
                print(io, "\n")
            end
            print(io, pad, "]")
        end
    elseif x === nothing
        print(io, "null")
    else
        print(io, JSON.json(x))
    end
    nothing
end

function fixture_text(sc, recs)::String
    io = IOBuffer()
    json_write(io, Pair{String,Any}["name" => sc.name, "steps" => recs], 0)
    print(io, "\n")
    String(take!(io))
end

scenario_runnable(sc) = !sc.needs_git || Sys.which("git") !== nothing

function record_all()::Nothing
    mkpath(CORPUS_DIR)
    for sc in SCENARIOS
        if !scenario_runnable(sc)
            println("skip $(sc.name) (git unavailable)")
            continue
        end
        recs = run_scenario(sc)
        write(joinpath(CORPUS_DIR, sc.name * ".json"), fixture_text(sc, recs))
        println("recorded $(sc.name) ($(length(recs)) steps)")
    end
    nothing
end

function print_drift(name, i, args, field, expected::String, actual::String)::Nothing
    println("DRIFT scenario=$name step=$i field=$field args=$(join(args, " "))")
    el = split(expected, '\n')
    al = split(actual, '\n')
    shown = 0
    for k in 1:max(length(el), length(al))
        e = k <= length(el) ? el[k] : nothing
        a = k <= length(al) ? al[k] : nothing
        e == a && continue
        shown += 1
        if shown > 20
            println("  ...")
            break
        end
        e === nothing || println("  - ", e)
        a === nothing || println("  + ", a)
    end
    nothing
end

function verify_scenario(sc, recs, fixed::String)::Bool
    d = JSON.parse(fixed)
    println("drift in scenario $(sc.name)")
    steps = d["steps"]
    for (i, rec) in enumerate(recs)
        i > length(steps) && break
        fs = steps[i]
        got = Dict{String,Any}(rec)
        String.(fs["args"]) == got["args"] ||
            println("  step $i args differ: fixture $(fs["args"]) live $(got["args"])")
        fs["exit"] == got["exit"] ||
            println("  step $i exit: expected $(fs["exit"]) got $(got["exit"])")
        for field in ("stdout", "stderr", "lock", "journal")
            want = fs[field] === nothing ? "" : fs[field]
            live = got[field] === nothing ? "" : got[field]
            want == live || print_drift(sc.name, i, fs["args"], field, want, live)
        end
    end
    length(recs) == length(steps) ||
        println("  step count: expected $(length(steps)) got $(length(recs))")
    false
end

function verify_all()::Bool
    ok = true
    n = 0
    for sc in SCENARIOS
        if !scenario_runnable(sc)
            println("skip $(sc.name) (git unavailable)")
            continue
        end
        n += 1
        fp = joinpath(CORPUS_DIR, sc.name * ".json")
        if !isfile(fp)
            println("missing fixture: $fp")
            ok = false
            continue
        end
        recs = run_scenario(sc)
        live = fixture_text(sc, recs)
        if live == read(fp, String)
            println("ok $(sc.name) ($(length(recs)) steps)")
        else
            ok = verify_scenario(sc, recs, read(fp, String))
        end
    end
    println(ok ? "conformance: $n scenarios green" : "conformance: drift detected")
    ok
end

function main(args)::Int
    mode = isempty(args) ? "verify" : args[1]
    mode == "record" && (record_all(); return 0)
    mode == "verify" && return verify_all() ? 0 : 1
    println(stderr, "usage: run.jl [record|verify]")
    1
end

end

if abspath(PROGRAM_FILE) == @__FILE__
    exit(Conformance.main(ARGS))
end
