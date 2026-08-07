user_grove_dir() = get(ENV, "GROVE_HOME", joinpath(homedir(), ".grove"))

registry_path() = joinpath(user_grove_dir(), "projects.toml")

mutable struct ProjectEntry
    name::String
    path::String
    created::String
    last_opened::String
end

function registry_load(path::AbstractString=registry_path())::Union{Nothing,Vector{ProjectEntry}}
    isfile(path) || return ProjectEntry[]
    local parsed
    try
        parsed = TOML.parse(read(path, String))
    catch
        return nothing
    end
    ps = get(parsed, "projects", nothing)
    ps === nothing && return ProjectEntry[]
    ps isa Vector || return nothing
    out = ProjectEntry[]
    for p in ps
        p isa AbstractDict || continue
        all(k -> get(p, k, nothing) isa AbstractString,
            ("name", "path", "created", "last_opened")) || continue
        push!(out, ProjectEntry(String(p["name"]), String(p["path"]),
                                String(p["created"]), String(p["last_opened"])))
    end
    out
end

toml_basic_string(s::AbstractString)::String =
    '"' * replace(replace(String(s), '\\' => "\\\\"), '"' => "\\\"") * '"'

function registry_save(entries::Vector{ProjectEntry}, path::AbstractString=registry_path())::Nothing
    isdir(dirname(path)) || mkpath(dirname(path))
    buf = IOBuffer()
    for e in entries
        println(buf, "[[projects]]")
        println(buf, "name = ", toml_basic_string(e.name))
        println(buf, "path = ", toml_basic_string(e.path))
        println(buf, "created = ", toml_basic_string(e.created))
        println(buf, "last_opened = ", toml_basic_string(e.last_opened))
        println(buf)
    end
    write(path, String(take!(buf)))
    nothing
end

function registry_unique_name(reg::Vector{ProjectEntry}, base::String)::String
    taken = Set(e.name for e in reg)
    base in taken || return base
    n = 2
    while string(base, '-', n) in taken
        n += 1
    end
    string(base, '-', n)
end

function registry_name_for_path(reg::Vector{ProjectEntry}, p::AbstractString)::Union{Nothing,String}
    ap = abspath(p)
    for e in reg
        e.path == ap && return e.name
    end
    nothing
end

function registry_path_for_name(reg::Vector{ProjectEntry}, name::AbstractString)::Union{Nothing,String}
    for e in reg
        e.name == name && return abspath(e.path)
    end
    nothing
end

function registry_note_open(root::AbstractString, cmd::AbstractString)::Nothing
    (cmd == "init" || isfile(joinpath(root, ".grove", "state.lock"))) || return nothing
    reg = registry_load()
    if reg === nothing
        println(stderr, "warning: malformed registry $(registry_path()); registry features disabled")
        return nothing
    end
    p = abspath(root)
    now = utc_stamp_second()
    i = findfirst(e -> e.path == p, reg)
    if i === nothing
        push!(reg, ProjectEntry(registry_unique_name(reg, basename(normpath(p))), p, now, now))
    else
        reg[i].last_opened = now
    end
    try
        registry_save(reg)
    catch
        println(stderr, "warning: could not write registry $(registry_path())")
    end
    nothing
end

function walk_up_root(start::AbstractString)::String
    dir = abspath(start)
    while true
        isfile(joinpath(dir, ".grove", "state.lock")) && return dir
        parent = dirname(dir)
        (parent == dir || isempty(parent)) && return abspath(start)
        dir = parent
    end
end

function resolve_project_target(v::AbstractString)::Union{Nothing,String}
    isdir(v) && return abspath(v)
    reg = registry_load()
    reg === nothing && (reg = ProjectEntry[])
    r = registry_path_for_name(reg, v)
    r !== nothing && return r
    println(stderr, "unknown project: $v")
    nothing
end

function resolve_root(default_root::AbstractString, kw::Dict{String,String},
                      root_given::Bool)::Union{Nothing,String}
    root_given && return default_root
    proj = get(kw, "project", nothing)
    if proj === nothing || isempty(strip(proj))
        env = get(ENV, "GROVE_PROJECT", nothing)
        proj = (env === nothing || isempty(strip(env))) ? nothing : env
    end
    proj === nothing && return walk_up_root(pwd())
    resolve_project_target(proj)
end
