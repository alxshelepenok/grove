#!/usr/bin/env julia

const ROOT = length(ARGS) >= 1 ? abspath(ARGS[1]) : abspath(joinpath(@__DIR__, "..", "..", "packages", "grove"))
const SCRATCH = mktempdir()
const WORKER = joinpath(@__DIR__, "worker.jl")

include(joinpath(@__DIR__, "trace_storage.jl"))

using Pkg
Pkg.activate(ROOT)

println(stderr, "[coverage/summary] SCRATCH=", SCRATCH, " ROOT=", ROOT)

try
    relocate_cov_traces!(ROOT)
    Base.withenv("JULIA_PKG_PRECOMPILE_AUTO" => "0") do
        Pkg.activate(SCRATCH)
        Pkg.add("Coverage"; io = stderr)
    end
    jc = Base.julia_cmd()
    ok = success(run(`$jc --project=$SCRATCH $WORKER $ROOT`; wait = true))
    ok || exit(1)
finally
    Pkg.activate(ROOT)
end
