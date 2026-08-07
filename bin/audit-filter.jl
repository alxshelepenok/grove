#!/usr/bin/env julia
import JSON

function fail(msg, code)
    println(stderr, "error: $msg")
    exit(code)
end

length(ARGS) == 2 || fail("usage: audit-filter.jl <trivy-report.json> <vex.json>", 2)

report = JSON.parsefile(ARGS[1])
vex = JSON.parsefile(ARGS[2])

suppressed = Dict{String,Vector{String}}()
for stmt in get(vex, "vulnerabilities", [])
    analysis = get(stmt, "analysis", Dict())
    get(analysis, "state", "") == "not_affected" || continue
    refs = [get(a, "ref", "") for a in get(stmt, "affects", [])]
    suppressed[get(stmt, "id", "")] = refs
end

function purl_name_version(purl)
    m = match(r"^pkg:[^/]+/([^@?]+)(?:@([^?]+))?(?:\?.*)?$", purl)
    m === nothing && return (nothing, nothing)
    return (lowercase(m.captures[1]), m.captures[2] === nothing ? nothing : lowercase(m.captures[2]))
end

function is_suppressed(id, pkg, ver)
    refs = get(suppressed, id, nothing)
    refs === nothing && return false
    pkg = lowercase(pkg)
    ver = lowercase(ver)
    for ref in refs
        rname, rver = purl_name_version(ref)
        if rname == pkg && rver !== nothing && rver == ver
            return true
        end
    end
    return false
end

fatal = []
warn = []
for result in get(report, "Results", [])
    target = get(result, "Target", "?")
    for v in get(result, "Vulnerabilities", [])
        id = get(v, "VulnerabilityID", "")
        pkg = get(v, "PkgName", "?")
        ver = get(v, "InstalledVersion", "?")
        sev = get(v, "Severity", "UNKNOWN")
        is_suppressed(id, pkg, ver) && continue
        if sev in ("CRITICAL", "HIGH")
            push!(fatal, (target, id, pkg, ver, sev))
        else
            push!(warn, (target, id, pkg, ver, sev))
        end
    end
end

for (target, id, pkg, ver, sev) in warn
    println(stderr, "warning: $sev $id $pkg@$ver ($target)")
end
for (target, id, pkg, ver, sev) in fatal
    println(stderr, "$sev $id $pkg@$ver ($target)")
end

if !isempty(fatal)
    println(stderr, "$(length(fatal)) CRITICAL/HIGH finding(s) not covered by VEX not_affected statements")
    exit(1)
end
if !isempty(warn)
    println(stderr, "$(length(warn)) lower-severity finding(s) reported as warnings (not blocking)")
end
println("no unsuppressed CRITICAL/HIGH findings")
