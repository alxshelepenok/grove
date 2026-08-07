#!/usr/bin/env julia
import JSON

function fail(msg)
    println(stderr, "error: $msg")
    exit(1)
end

length(ARGS) == 1 || fail("usage: validate-vex.jl <vex.json>")
isfile(ARGS[1]) || fail("file not found: $(ARGS[1])")

doc = JSON.parsefile(ARGS[1])

get(doc, "bomFormat", nothing) == "CycloneDX" || fail("bomFormat must be \"CycloneDX\"")
get(doc, "specVersion", nothing) == "1.6" || fail("specVersion must be \"1.6\"")
version = get(doc, "version", nothing)
version isa Integer && version >= 1 || fail("version must be an integer >= 1")
vulns = get(doc, "vulnerabilities", nothing)
vulns isa Vector || fail("vulnerabilities must be an array")

id_re = r"^(CVE-\d{4}-\d{4,}|GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}|JLSEC-\d{4}-\d+|RUSTSEC-\d{4}-\d{4,})$"
states = Set(["affected", "not_affected", "fixed", "under_investigation"])
justifications = Set([
    "code_not_present", "code_not_reachable", "requires_configuration",
    "requires_dependency", "requires_environment", "protected_by_compiler",
    "protected_at_runtime", "protected_at_perimeter",
    "protected_by_mitigating_control",
])

for (i, stmt) in enumerate(vulns)
    stmt isa AbstractDict || fail("vulnerabilities[$i] must be an object")
    id = get(stmt, "id", nothing)
    id isa AbstractString && occursin(id_re, id) || fail("vulnerabilities[$i].id is missing or malformed: $id")
    affects = get(stmt, "affects", nothing)
    affects isa Vector && !isempty(affects) || fail("vulnerabilities[$i].affects must be a non-empty array")
    for (j, aff) in enumerate(affects)
        ref = aff isa AbstractDict ? get(aff, "ref", nothing) : nothing
        ref isa AbstractString && !isempty(ref) || fail("vulnerabilities[$i].affects[$j].ref is missing")
    end
    analysis = get(stmt, "analysis", nothing)
    analysis isa AbstractDict || fail("vulnerabilities[$i].analysis must be an object")
    state = get(analysis, "state", nothing)
    state in states || fail("vulnerabilities[$i].analysis.state must be one of $(join(states, ", "))")
    justification = get(analysis, "justification", nothing)
    if justification !== nothing
        justification in justifications || fail("vulnerabilities[$i].analysis.justification is not a known value: $justification")
    end
    detail = get(analysis, "detail", nothing)
    if state == "not_affected" && justification === nothing && (detail === nothing || isempty(detail))
        fail("vulnerabilities[$i] is not_affected but has neither justification nor detail")
    end
end

println("OK: $(length(vulns)) statement(s) valid")
