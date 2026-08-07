#!/usr/bin/env julia
import JSON
import Dates

function uuid4()
    bytes = rand(UInt8, 16)
    bytes[7] = (bytes[7] & 0x0f) | 0x40
    bytes[9] = (bytes[9] & 0x3f) | 0x80
    h = lowercase(bytes2hex(bytes))
    "$(h[1:8])-$(h[9:12])-$(h[13:16])-$(h[17:20])-$(h[21:32])"
end

function fail(msg)
    println(stderr, "error: $msg")
    exit(1)
end

length(ARGS) >= 2 || fail("usage: merge-cdx.jl <output> <input.cdx.json>...")

output = ARGS[1]
inputs = ARGS[2:end]

components = Dict{String,Any}()
dependencies = Dict{String,Any}()

compkey(c) = get(c, "purl", get(c, "bom-ref", "$(get(c, "name", ""))@$(get(c, "version", ""))"))

for f in inputs
    isfile(f) || fail("input not found: $f")
    d = JSON.parsefile(f)
    for c in get(d, "components", [])
        get(c, "name", "") == "Manifest.toml" && continue
        components[compkey(c)] = c
    end
    metadata = get(d, "metadata", Dict())
    mc = get(metadata, "component", nothing)
    if mc !== nothing && get(mc, "type", "") == "application"
        components[compkey(mc)] = mc
    end
    for dep in get(d, "dependencies", [])
        ref = get(dep, "ref", "")
        isempty(ref) && continue
        dependencies[ref] = get(dep, "dependsOn", [])
    end
end

sorted_components = sort(collect(values(components)); by=compkey)
sorted_dependencies = [
    Dict("ref" => ref, "dependsOn" => dependencies[ref])
    for ref in sort(collect(keys(dependencies)))
]

timestamp = Dates.format(Dates.now(Dates.UTC), "yyyy-mm-ddTHH:MM:SSZ")

doc = Dict(
    "bomFormat" => "CycloneDX",
    "specVersion" => "1.6",
    "serialNumber" => "urn:uuid:$(uuid4())",
    "version" => 1,
    "metadata" => Dict(
        "timestamp" => timestamp,
        "tools" => [
            Dict("name" => "cargo-cyclonedx"),
            Dict("name" => "trivy"),
        ],
        "component" => Dict(
            "type" => "application",
            "name" => "grove",
            "bom-ref" => "pkg:generic/grove",
        ),
    ),
    "components" => sorted_components,
    "dependencies" => sorted_dependencies,
)

open(output, "w") do io
    JSON.print(io, doc, 2)
    println(io)
end

println("merged $(length(inputs)) documents: $(length(sorted_components)) components, $(length(sorted_dependencies)) dependency entries")
