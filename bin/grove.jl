#!/usr/bin/env julia
import Pkg
const ROOT = normpath(joinpath(@__DIR__, "..", "packages", "grove"))
Pkg.activate(ROOT; io=devnull)
using grove
exit(grove.main(ARGS))
