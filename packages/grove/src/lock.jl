const MAGIC = "@grove 1"
const HEADER_COMMENT = "# AUTO-GENERATED. Do not edit. Use `grove` CLI."
const CHECKSUM_PREFIX = "# checksum: sha256:"

const FIELD_CATALOG = Dict{Tuple{Symbol,Symbol},Symbol}(
    (:w, :tags) => :reflist, (:d, :tags) => :reflist, (:q, :tags) => :reflist,
    (:b, :tags) => :reflist, (:g, :tags) => :reflist,
    # W.
    (:w, :goals) => :reflist,
    (:w, :theme) => :single,
    (:w, :fitness) => :fitness,
    (:w, :surface) => :reflist,
    (:w, :ac) => :prose,
    (:w, :hypothesis) => :prose,
    (:w, :repro) => :prose,
    (:w, :exit) => :prose,
    (:w, :evidence_strategy) => :prose,
    (:w, :evidence) => :prose,
    (:w, :plan) => :prose,
    (:w, :why) => :prose,
    # D.
    (:d, :context) => :prose,
    (:d, :options) => :prose,
    (:d, :decision) => :prose,
    (:d, :consequences) => :prose,
    (:d, :validation) => :prose,
    # Q.
    (:q, :why) => :prose,
    (:q, :hypothesis) => :prose,
    (:q, :exit) => :prose,
    (:q, :log) => :prose,
    (:q, :outcome) => :prose,
    # B.
    (:b, :vm) => :prose,
    (:b, :threshold) => :prose,
    (:b, :result) => :prose,
    # G.
    (:g, :area) => :single,
    (:g, :notes) => :prose,
    (:g, :fitness_target) => :single,
    (:g, :fitness_current) => :single,
    # A.
    (:t, :tags) => :reflist,
    (:t, :notes) => :prose,
    (:y, :tags) => :reflist,
    (:y, :surface) => :reflist,
    (:y, :invariant) => :prose,
    (:y, :why) => :prose,
    (:y, :skill_updates) => :prose,
    (:y, :glossary_updates) => :prose,
    (:y, :revalidation) => :prose,
    (:a, :surface) => :reflist,
)

const FIELD_ORDER = Dict{Symbol,Vector{Symbol}}(
    :w => [:goals, :theme, :fitness, :tags, :surface, :ac, :hypothesis, :repro, :exit, :evidence_strategy, :evidence, :plan, :why],
    :d => [:tags, :context, :options, :decision, :consequences, :validation],
    :q => [:tags, :why, :hypothesis, :exit, :log, :outcome],
    :b => [:tags, :vm, :threshold, :result],
    :g => [:area, :tags, :fitness_target, :fitness_current, :notes],
    :t => [:tags, :notes],
    :y => [:tags, :surface, :invariant, :why, :skill_updates, :glossary_updates, :revalidation],
    :a => [:surface],
)

needs_quote(s::AbstractString) = isempty(s) ||
                                 any(c -> c == ' ' || c == '"' || c == '\\' || c == '\t' || c == '\n', s)

function quote_str(s::AbstractString)::String
    buf = IOBuffer()
    print(buf, '"')
    for c in s
        if c == '"'
            print(buf, "\\\"")
        elseif c == '\\'
            print(buf, "\\\\")
        elseif c == '\n'
            print(buf, "\\n")
        else
            print(buf, c)
        end
    end
    print(buf, '"')
    String(take!(buf))
end

maybe_quote(s::AbstractString) = needs_quote(s) ? quote_str(s) : String(s)

function parse_qstring(s::AbstractString, i::Int)::Tuple{String,Int}
    @assert s[i] == '"'
    i += 1
    buf = IOBuffer()
    while i <= lastindex(s)
        c = s[i]
        if c == '"'
            return String(take!(buf)), i + 1
        elseif c == '\\' && i + 1 <= lastindex(s)
            nc = s[i+1]
            if nc == '"'
                print(buf, '"')
                i += 2
            elseif nc == '\\'
                print(buf, '\\')
                i += 2
            elseif nc == 'n'
                print(buf, '\n')
                i += 2
            else
                error("bad escape \\$nc")
            end
        else
            print(buf, c)
            i = nextind(s, i)
        end
    end
    error("unterminated quoted string")
end

function tokenize_header(line::AbstractString)::Vector{Tuple}
    tokens = Tuple[]
    i = 1
    n = lastindex(line)
    while i <= n
        c = line[i]
        if c == ' ' || c == '\t'
            i = nextind(line, i)
            continue
        elseif c == '"'
            s, j = parse_qstring(line, i)
            push!(tokens, ("str", s))
            i = j
        else
            buf = IOBuffer()
            while i <= n && line[i] != ' ' && line[i] != '\t'
                if line[i] == '"'
                    s, j = parse_qstring(line, i)
                    print(buf, quote_str(s))
                    i = j
                else
                    print(buf, line[i])
                    i = nextind(line, i)
                end
            end
            tok = String(take!(buf))
            eqpos = findfirst('=', tok)
            if eqpos !== nothing
                key = tok[1:eqpos-1]
                rest = tok[eqpos+1:end]
                if !isempty(rest) && rest[1] == '"'
                    s, _ = parse_qstring(rest, 1)
                    push!(tokens, ("eq", key, s))
                else
                    push!(tokens, ("eq", key, rest))
                end
            else
                push!(tokens, ("bare", tok))
            end
        end
    end
    tokens
end

struct LockParseError <: Exception
    msg::String
    line::Int
end
Base.showerror(io::IO, e::LockParseError) = print(io, "lock parse error at line ", e.line, ": ", e.msg)

"""
    parse_lock(text) -> (state::State, body::String, expected_checksum::String)

Parses the lock file content. Does not verify checksum; caller does.
"""
function parse_lock(text::AbstractString)::Tuple{State,String,String}
    lines = split(text, '\n')
    st = State()
    expected = ""
    body_start = 0

    if length(lines) < 3
        throw(LockParseError("file too short", 0))
    end
    magic = strip(lines[1])
    if magic != MAGIC
        throw(LockParseError("missing magic '$MAGIC'", 1))
    end
    if !startswith(lines[2], "#")
        throw(LockParseError("missing header comment", 2))
    end
    if !startswith(lines[3], CHECKSUM_PREFIX)
        throw(LockParseError("missing checksum", 3))
    end
    expected = strip(lines[3][lastindex(CHECKSUM_PREFIX)+1:end])
    body_start = 4

    while body_start <= length(lines) && isempty(lines[body_start])
        body_start += 1
    end
    body = join(lines[body_start:end], '\n')

    scan_from = consume_lock_id_meta!(st, lines, body_start)

    in_archive = false
    cur_node::Union{Nothing,Node} = nothing
    cur_field::Union{Nothing,Symbol} = nothing
    i = scan_from
    while i <= length(lines)
        raw = lines[i]
        ln = i
        i += 1
        if isempty(raw)
            cur_field = nothing
            continue
        end
        if startswith(raw, "# ")
            cur_field = nothing
            continue
        end
        if raw == ":archive"
            in_archive = true
            cur_node = nothing
            cur_field = nothing
            continue
        end
        if startswith(raw, "    | ") || raw == "    |"
            cur_node === nothing && throw(LockParseError("prose without record", ln))
            cur_field === nothing && throw(LockParseError("prose without field", ln))
            text = length(raw) > 6 ? raw[7:end] : ""
            push!(get!(cur_node.fields, cur_field, String[]), text)
            continue
        end
        if startswith(raw, "  ") && !startswith(raw, "   ") && length(raw) > 2 && raw[3] != ' '
            cur_node === nothing && throw(LockParseError("field without record", ln))
            colon = findfirst(':', raw)
            colon === nothing && throw(LockParseError("missing ':' in field", ln))
            key = strip(raw[3:colon-1])
            value = colon < lastindex(raw) ? strip(raw[colon+1:end]) : ""
            fsym = Symbol(key)
            form = get(FIELD_CATALOG, (cur_node.kind, fsym), nothing)
            form === nothing && throw(LockParseError("unknown field '$key' on $(cur_node.kind)", ln))
            cur_field = fsym
            if form === :prose
                cur_node.fields[fsym] = String[]
                if !isempty(value)
                    throw(LockParseError("prose field must not have inline value", ln))
                end
            elseif form === :reflist
                cur_node.fields[fsym] = isempty(value) ? String[] :
                                        [strip(s) for s in split(value, ',')]
            elseif form === :single
                cur_node.fields[fsym] = isempty(value) ? "" : value
            elseif form === :fitness
                d = Dict{String,Int}()
                if !isempty(value)
                    for part in split(value, ',')
                        part = strip(part)
                        eq = findfirst('=', part)
                        eq === nothing && throw(LockParseError("bad fitness entry '$part'", ln))
                        gid = strip(part[1:eq-1])
                        delta = parse(Int, strip(part[eq+1:end]))
                        d[gid] = delta
                    end
                end
                cur_node.fields[fsym] = d
            end
            continue
        end
        cur_field = nothing
        toks = tokenize_header(raw)
        isempty(toks) && continue
        first = toks[1]
        first[1] == "bare" || throw(LockParseError("expected record kind", ln))
        kindstr = first[2]
        if kindstr == "e"
            length(toks) >= 4 || throw(LockParseError("malformed edge", ln))
            from = toks[2][2]
            label = toks[3][2]
            to = toks[4][2]
            tc = nothing
            for k in 5:length(toks)
                t = toks[k]
                t[1] == "eq" || throw(LockParseError("unexpected edge token", ln))
                if t[2] == "t_created"
                    tc === nothing || throw(LockParseError("duplicate t_created on edge", ln))
                    tc = t[3]
                else
                    throw(LockParseError("unknown edge attribute '$(t[2])' (only t_created allowed)", ln))
                end
            end
            push!(st.edges, Edge(String(from), Symbol(label), String(to), tc))
            record_id!(st, from)
            record_id!(st, to)
            cur_node = nothing
            continue
        end
        kind = Symbol(kindstr)
        kind in NODE_KINDS || throw(LockParseError("unknown record kind '$kindstr'", ln))
        (in_archive && kind === :y) && throw(LockParseError("y record in :archive section", ln))
        length(toks) >= 2 || throw(LockParseError("missing id", ln))
        toks[2][1] == "bare" || throw(LockParseError("malformed id", ln))
        id = toks[2][2]
        record_id!(st, id)
        n = Node(kind, id)
        n.archived = in_archive
        for t in toks[3:end]
            if t[1] == "eq"
                k = t[2]
                v = t[3]
                if k == "type"
                    n.type = Symbol(v)
                elseif k == "status"
                    n.status = Symbol(v)
                elseif k == "cynefin"
                    n.cynefin = Symbol(v)
                else
                    n.attrs[k] = v
                end
            elseif t[1] == "str"
                n.title = t[2]
            elseif t[1] == "bare"
                throw(LockParseError("unexpected token '$(t[2])'", ln))
            end
        end
        if kind === :t && n.status === :proposed
            n.status = :open
        end
        if kind === :a && n.status !== :present
            throw(LockParseError("a record status must be present (got '$(n.status)')", ln))
        end
        st.nodes[id] = n
        cur_node = n
    end
    st, body, expected
end

"""
    consume_lock_id_meta!(st, lines, i0) -> i

If `lines[i0:...]` begins with optional blank lines then a `# @grove-id stride=…` line,
apply to `st` and return the index of the first line that is not consumed.
"""
function consume_lock_id_meta!(st::State, lines::AbstractVector{<:AbstractString}, i0::Int)::Int
    j = i0
    while j <= length(lines)
        raw = lines[j]
        if isempty(strip(raw))
            j += 1
            continue
        end
        m = match(r"^#\s+@grove-id\s+stride=(\d+)\s+offset=(\d+)\s+pad=(\d+)\s*$", raw)
        if m !== nothing
            st.id_stride = parse(Int, m[1])
            st.id_offset = parse(Int, m[2])
            st.id_pad_width = parse(Int, m[3])
            j += 1
            continue
        end
        break
    end
    return j
end

"""
    serialize_body(state) -> String

"""
function serialize_body(st::State)::String
    io = IOBuffer()
    if !(st.id_stride == 1 && st.id_offset == 1 && st.id_pad_width == 2)
        print(io, "# @grove-id stride=", st.id_stride, " offset=", st.id_offset,
            " pad=", st.id_pad_width, "\n\n")
    end
    for archived in (false, true)
        first_in_section = true
        if archived
            any(n -> n.archived, values(st.nodes)) || break
            print(io, position(io) == 0 ? ":archive\n" : "\n:archive\n")
        end
        for kind in NODE_KINDS
            for n in listnodes(st, kind; include_archived=true)
                n.archived == archived || continue
                if !first_in_section
                    print(io, "\n")
                end
                first_in_section = false
                serialize_node!(io, n)
            end
        end
        if !archived
            edges = sort(st.edges; by=e -> (e.from, String(e.label), e.to))
            if !isempty(edges)
                print(io, "\n")
                for e in edges
                    print(io, "e ", e.from, " ", String(e.label), " ", e.to)
                    tc = e.t_created === nothing ? "" : String(e.t_created)
                    print(io, " t_created=", maybe_quote(tc), "\n")
                end
            end
        end
    end
    String(take!(io))
end

function serialize_node!(io::IO, n::Node)
    print(io, String(n.kind), " ", n.id)
    if n.kind === :w
        n.type === nothing || print(io, " type=", String(n.type))
        print(io, " status=", String(n.status))
        n.cynefin === nothing || print(io, " cynefin=", String(n.cynefin))
    elseif n.kind === :g
        print(io, " status=", String(n.status))
        if haskey(n.attrs, "fitness")
            print(io, " fitness=", maybe_quote(n.attrs["fitness"]))
        end
    elseif n.kind === :d
        print(io, " status=", String(n.status))
    elseif n.kind === :q || n.kind === :b
        print(io, " status=", String(n.status))
        n.cynefin === nothing || print(io, " cynefin=", String(n.cynefin))
    elseif n.kind === :t
        print(io, " status=", String(n.status))
    elseif n.kind === :y
        print(io, " status=", String(n.status))
    elseif n.kind === :a
        print(io, " status=", String(n.status))
    end
    skip = Set(["fitness", "goal", "date"])
    for k in sort!(collect(keys(n.attrs)))
        k in skip && continue
        print(io, " ", k, "=", maybe_quote(n.attrs[k]))
    end
    if !isempty(n.title)
        print(io, " ", quote_str(n.title))
    end
    print(io, "\n")
    order = get(FIELD_ORDER, n.kind, Symbol[])
    for fsym in order
        haskey(n.fields, fsym) || continue
        v = n.fields[fsym]
        form = FIELD_CATALOG[(n.kind, fsym)]
        if form === :prose
            isempty(v) && continue
            print(io, "  ", String(fsym), ":\n")
            for line in v
                print(io, "    | ", line, "\n")
            end
        elseif form === :reflist
            isempty(v) && continue
            print(io, "  ", String(fsym), ": ", join(v, ", "), "\n")
        elseif form === :single
            isempty(v) && continue
            print(io, "  ", String(fsym), ": ", v, "\n")
        elseif form === :fitness
            isempty(v) && continue
            parts = String[]
            for k in sort!(collect(keys(v)))
                d = v[k]
                push!(parts, string(k, "=", d >= 0 ? "+" : "", d))
            end
            print(io, "  ", String(fsym), ": ", join(parts, ", "), "\n")
        end
    end
end

checksum_of(body::AbstractString)::String =
    bytes2hex(sha256(codeunits(String(body))))

"""
Stage `payload` in a tempfile under `dirname(path)`, flush it, then `mv` onto `path`.

`mv(...; force=isfile(dest))` matches Base semantics: POSIX atomic replace where supported;
Windows uses rename-first with delete/fallback (`Base.Filesystem`). Observers either see the previous contents or the full new blob, not a truncate-in-progress artifact at `path`.

On write failure before `mv`, an existing destination file stays unchanged; the tempfile is removed in `finally`."""
function atomic_write_same_dir!(dst::AbstractString, payload::AbstractString)::Nothing
    path = abspath(String(dst))
    d = dirname(path)
    mkpath(d)
    tmp_path, io = mktemp(d)
    moved = false
    try
        print(io, payload)
        flush(io)
        close(io)
        io = nothing
        mv(tmp_path, path; force=isfile(path))
        moved = true
    finally
        if io !== nothing
            try
                close(io)
            catch
            end
        end
        !moved && ispath(tmp_path) && rm(tmp_path; force=true)
    end
    nothing
end

"""
    write_lock(path, state)

Serializes `state`, recomputes the checksum, writes atomically with `atomic_write_same_dir!`.
"""
function write_lock(path::AbstractString, st::State)
    body = serialize_body(st)
    cks = checksum_of(body)
    buf = IOBuffer()
    print(buf, MAGIC, "\n")
    print(buf, HEADER_COMMENT, "\n")
    print(buf, CHECKSUM_PREFIX, cks, "\n")
    print(buf, body)
    endswith(body, "\n") || print(buf, "\n")
    atomic_write_same_dir!(path, String(take!(buf)))
end

"""
    read_lock(path; verify=true) -> State

Reads the lock and verifies its checksum. Throws on mismatch unless `verify=false`.
"""
function read_lock(path::AbstractString; verify::Bool=true)::State
    isfile(path) || error("lock not found: $path")
    text = read(path, String)
    text = replace(text, "\r\n" => "\n")
    st, body, expected = parse_lock(text)
    if verify
        actual = checksum_of(body)
        if actual != expected
            throw(ChecksumMismatch(expected, actual))
        end
    end
    st
end

struct ChecksumMismatch <: Exception
    expected::String
    actual::String
end
Base.showerror(io::IO, e::ChecksumMismatch) =
    print(io, "lock checksum mismatch (expected ", e.expected, ", got ", e.actual,
        "). Did you edit state.lock by hand? Run `grove repair --confirm` to accept the current contents.")
