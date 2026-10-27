using Test
using grove
using JSON
const M = grove
tdiag(x) = x
tdiag(d::grove.Diagnostic) = d.text
ENV["GROVE_HOME"] = mktempdir()
delete!(ENV, "GROVE_PROJECT")
f = only(ARGS)
Base.include(Main, f)
