struct DiagCode
    name::String
    function DiagCode(name::String)
        name in ("SYSTEM", "INTERFACE", "INVARIANT", "OPERATIONAL") ||
            throw(ArgumentError("bad diagnostic code $(repr(name))"))
        new(name)
    end
end

const CODE_SYSTEM = DiagCode("SYSTEM")
const CODE_INTERFACE = DiagCode("INTERFACE")
const CODE_INVARIANT = DiagCode("INVARIANT")
const CODE_OPERATIONAL = DiagCode("OPERATIONAL")

const DIAG_CODES = (CODE_SYSTEM, CODE_INTERFACE, CODE_INVARIANT, CODE_OPERATIONAL)

Base.string(c::DiagCode) = c.name

const DIAG_FORBIDDEN_CHARS = (';', ':', '\u2014')

function diag_code_from_name(s::AbstractString)::Union{DiagCode,Nothing}
    for c in DIAG_CODES
        c.name == String(s) && return c
    end
    return nothing
end

function contains_cli_invocation(text::AbstractString)::Bool
    s = String(text)
    i = firstindex(s)
    while true
        pos = findnext("grove", s, i)
        pos === nothing && return false
        start = pos[1]
        stop = start + length("grove") - 1
        after_ok = stop >= lastindex(s) || !(isnumeric(s[nextind(s, stop)]) || isletter(s[nextind(s, stop)]))
        before_ok = start == firstindex(s) || !(isnumeric(s[prevind(s, start)]) || isletter(s[prevind(s, start)]) || s[prevind(s, start)] == '-')
        if after_ok && before_ok
            j = nextind(s, stop)
            while j <= lastindex(s) && (s[j] == '`' || s[j] == '"' || isspace(s[j]))
                j = nextind(s, j)
            end
            tok = IOBuffer()
            k = j
            while k <= lastindex(s) && (islowercase(s[k]) || s[k] == '_' || s[k] == '-')
                write(tok, s[k])
                k = nextind(s, k)
            end
            token = String(take!(tok))
            if !isempty(token) && haskey(COMMANDS, token)
                return true
            end
        end
        i = nextind(s, start)
    end
end

function validate_diag_text(text::AbstractString)::Nothing
    t = String(text)
    isempty(strip(t)) && throw(ArgumentError("text is empty"))
    for c in t
        c in DIAG_FORBIDDEN_CHARS && throw(ArgumentError("text contains forbidden character $(escape_string(string(c)))"))
    end
    contains_cli_invocation(t) && throw(ArgumentError("text invokes a CLI command"))
    nothing
end

struct Diagnostic
    severity::String
    code::DiagCode
    text::String
    details::Vector{Pair{String,String}}
end

function diagnostic(severity::AbstractString, code::DiagCode, text::AbstractString)::Diagnostic
    sev = String(severity)
    sev in ("error", "warning", "info") ||
        throw(ArgumentError("bad diagnostic severity $(repr(sev))"))
    t = String(text)
    validate_diag_text(t)
    Diagnostic(sev, code, t, Pair{String,String}[])
end

function diagnostic(severity::AbstractString, code::DiagCode, text::AbstractString,
                    details::Vector{Pair{String,String}})::Diagnostic
    sev = String(severity)
    sev in ("error", "warning", "info") ||
        throw(ArgumentError("bad diagnostic severity $(repr(sev))"))
    t = String(text)
    validate_diag_text(t)
    Diagnostic(sev, code, t, details)
end

diag_error(code::DiagCode, text::AbstractString) = diagnostic("error", code, text)
diag_warning(code::DiagCode, text::AbstractString) = diagnostic("warning", code, text)
diag_info(code::DiagCode, text::AbstractString) = diagnostic("info", code, text)
diag_error(code::DiagCode, text::AbstractString, details::Vector{Pair{String,String}}) =
    diagnostic("error", code, text, details)
diag_warning(code::DiagCode, text::AbstractString, details::Vector{Pair{String,String}}) =
    diagnostic("warning", code, text, details)
diag_info(code::DiagCode, text::AbstractString, details::Vector{Pair{String,String}}) =
    diagnostic("info", code, text, details)

system_error(text::AbstractString) = diag_error(CODE_SYSTEM, text)
system_warning(text::AbstractString) = diag_warning(CODE_SYSTEM, text)
interface_error(text::AbstractString) = diag_error(CODE_INTERFACE, text)
invariant_error(text::AbstractString) = diag_error(CODE_INVARIANT, text)
operational_info(text::AbstractString) = diag_info(CODE_OPERATIONAL, text)
operational_warning(text::AbstractString) = diag_warning(CODE_OPERATIONAL, text)
system_error(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_error(CODE_SYSTEM, text, details)
system_warning(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_warning(CODE_SYSTEM, text, details)
interface_error(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_error(CODE_INTERFACE, text, details)
invariant_error(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_error(CODE_INVARIANT, text, details)
operational_info(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_info(CODE_OPERATIONAL, text, details)
operational_warning(text::AbstractString, details::Vector{Pair{String,String}}) =
    diag_warning(CODE_OPERATIONAL, text, details)

function raw_diag(severity::AbstractString, code::DiagCode, text::AbstractString)::Diagnostic
    sev = String(severity)
    sev in ("error", "warning", "info") ||
        throw(ArgumentError("bad diagnostic severity $(repr(sev))"))
    Diagnostic(sev, code, String(text), Pair{String,String}[])
end

function render_diag(d::Diagnostic)::String
    io = IOBuffer()
    print(io, d.severity, ' ', d.code.name, ' ', d.text)
    for (k, v) in d.details
        print(io, '\n', "  ", k, '=', v)
    end
    String(take!(io))
end

function json_escape_diag(s::AbstractString)::String
    io = IOBuffer()
    for c in String(s)
        if c == '"'
            write(io, "\\\"")
        elseif c == '\\'
            write(io, "\\\\")
        elseif c == '\n'
            write(io, "\\n")
        elseif c == '\r'
            write(io, "\\r")
        elseif c == '\t'
            write(io, "\\t")
        elseif c < ' '
            print(io, "\\u", lpad(string(UInt32(c); base = 16), 4, '0'))
        else
            write(io, c)
        end
    end
    String(take!(io))
end

function diag_json_str(d::Diagnostic)::String
    io = IOBuffer()
    print(io, "{\"severity\":\"", json_escape_diag(d.severity),
        "\",\"code\":\"", json_escape_diag(d.code.name),
        "\",\"text\":\"", json_escape_diag(d.text), "\"")
    if !isempty(d.details)
        print(io, ",\"details\":{")
        for (i, (k, v)) in enumerate(d.details)
            i > 1 && print(io, ",")
            print(io, '"', json_escape_diag(k), "\":\"", json_escape_diag(v), '"')
        end
        print(io, '}')
    end
    print(io, '}')
    String(take!(io))
end

"""Insertion-ordered string map so JSON field order matches the Rust emitter."""
struct StrMap{V} <: AbstractDict{String,V}
    kvs::Vector{Pair{String,V}}
end

StrMap(kvs::Vector{Pair{String,String}}) = StrMap{String}(kvs)

Base.length(m::StrMap) = length(m.kvs)
Base.iterate(m::StrMap, i::Int = 1) = i <= length(m.kvs) ? (m.kvs[i], i + 1) : nothing
Base.haskey(m::StrMap, k::AbstractString) = any(p -> p.first == String(k), m.kvs)
function Base.getindex(m::StrMap, k::AbstractString)
    i = findfirst(p -> p.first == String(k), m.kvs)
    i === nothing && throw(KeyError(String(k)))
    return m.kvs[i].second
end
Base.get(m::StrMap, k::AbstractString, def) = begin
    i = findfirst(p -> p.first == String(k), m.kvs)
    i === nothing ? def : m.kvs[i].second
end
Base.keys(m::StrMap) = String[p.first for p in m.kvs]
Base.values(m::StrMap) = Any[p.second for p in m.kvs]

function diag_json_obj(d::Diagnostic)::StrMap
    kvs = Pair{String,Any}[
        "severity" => String(d.severity),
        "code" => d.code.name,
        "text" => d.text,
    ]
    isempty(d.details) || push!(kvs, "details" => StrMap(Pair{String,String}[String(k) => String(v) for (k, v) in d.details]))
    StrMap{Any}(kvs)
end
