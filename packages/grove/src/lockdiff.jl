function edge_semantic_key(e::Edge)::Tuple{String,String,String}
    (e.from, String(e.label), e.to)
end

function edge_multiset_counts(st::State)::Dict{Tuple{String,String,String},Int}
    d = Dict{Tuple{String,String,String},Int}()
    for e in st.edges
        k = edge_semantic_key(e)
        d[k] = get(d, k, 0) + 1
    end
    d
end

function multiset_diff(a::Dict{K,Int}, b::Dict{K,Int}) where {K}
    outs = Tuple{Int,K}[]
    keys_all = sort!(collect(union(Set(keys(a)), Set(keys(b)))))
    for k in keys_all
        da = get(a, k, 0)
        db = get(b, k, 0)
        da == db && continue
        if da > db
            for _ = 1:(da-db)
                push!(outs, (-1, k))
            end
        else
            for _ = 1:(db-da)
                push!(outs, (1, k))
            end
        end
    end
    sort!(outs)
    outs
end

function sorted_dict_equal(a::Dict{String,String}, b::Dict{String,String})::Bool
    length(a) == length(b) || return false
    ka = sort(collect(keys(a)))
    kb = sort(collect(keys(b)))
    ka == kb || return false
    all(k -> a[k] == b[k], ka)
end

function reflist_semantic_equal(va::AbstractVector, vb::AbstractVector)::Bool
    isempty(va) && isempty(vb) && return true
    sort(collect(String.(va))) == sort(collect(String.(vb)))
end

function prose_semantic_equal(va::AbstractVector, vb::AbstractVector)::Bool
    isempty(va) && isempty(vb) && return true
    sort(collect(String.(va))) == sort(collect(String.(vb)))
end

function field_semantically_equal(kind::Symbol, fname::Symbol,
    a::Dict{Symbol,Any}, b::Dict{Symbol,Any})::Bool
    form = FIELD_CATALOG[(kind, fname)]
    va = get(a, fname, nothing)
    vb = get(b, fname, nothing)
    if form === :prose
        pa = va === nothing ? String[] : String.(va)::Vector{String}
        pb = vb === nothing ? String[] : String.(vb)::Vector{String}
        return prose_semantic_equal(pa, pb)
    elseif form === :reflist
        ra = va === nothing ? String[] : String.(va)::Vector{String}
        rb = vb === nothing ? String[] : String.(vb)::Vector{String}
        return reflist_semantic_equal(ra, rb)
    elseif form === :single
        sa = va === nothing ? "" : string(va)::String
        sb = vb === nothing ? "" : string(vb)::String
        return sa == sb
    elseif form === :fitness
        da::Dict{String,Int} = va === nothing ? Dict{String,Int}() : va
        db::Dict{String,Int} = vb === nothing ? Dict{String,Int}() : vb
        return da == db
    end
    false
end

function attrs_semantically_equal(a::Node, b::Node)::Bool
    sorted_dict_equal(a.attrs, b.attrs)
end

"""True when two nodes carry the same meaning (ordering of prose bullets ignored)."""
function node_semantically_equal(a::Node, b::Node)::Bool
    a.kind === b.kind &&
        a.title == b.title &&
        a.type == b.type &&
        a.status === b.status &&
        a.cynefin == b.cynefin &&
        a.archived == b.archived &&
        attrs_semantically_equal(a, b) &&
        all(fname -> field_semantically_equal(a.kind, fname, a.fields, b.fields),
            FIELD_ORDER[a.kind])
end

fmt_fitness_dict(d::Dict{String,Int})::String =
    isempty(d) ? "" :
    join([string(k, "=", v >= 0 ? "+" : "", v) for (k, v) in sort(collect(d))], ", ")

function fmt_single_field(kind::Symbol, fname::Symbol, v::Any)::String
    form = FIELD_CATALOG[(kind, fname)]
    if form === :prose
        lines::Vector{String} = v isa AbstractVector ? String.(v) : String[]
        isempty(lines) ? "(empty)" : string(length(lines), " prose lines")
    elseif form === :reflist
        xs::Vector{String} = v isa AbstractVector ? String.(v) : String[]
        isempty(xs) ? "(empty)" : join(sort(xs), ",")
    elseif form === :single
        string(v)::String
    elseif form === :fitness
        fmt_fitness_dict(v::Dict{String,Int})
    else
        repr(v)
    end
end

function node_field_snap(kind::Symbol, fields::Dict{Symbol,Any}, fname::Symbol)::String
    v = get(fields, fname, nothing)
    typ = FIELD_CATALOG[(kind, fname)]
    if v === nothing
        if typ === :prose || typ === :reflist
            return "(empty)"
        end
        return ""
    end
    fmt_single_field(kind, fname, v)
end

function discrete_header_line(n::Node)::String
    parts = String[]
    push!(parts, "$(n.kind) $(n.id)")
    if n.kind === :w
        push!(parts, string(n.status))
        n.type === nothing || push!(parts, string(n.type))
    elseif n.kind === :g
        push!(parts, string(n.status))
    elseif n.kind in (:q, :b)
        push!(parts, string(n.status))
        n.cynefin === nothing || push!(parts, string(n.cynefin))
    elseif n.kind === :d
        push!(parts, string(n.status))
    elseif n.kind === :t
        push!(parts, string(n.status))
    elseif n.kind === :y
        push!(parts, string(n.status))
    end
    join(parts, " ")
end

function describe_node_changes(a::Node, b::Node)::Vector{String}
    out = String[]
    na = discrete_header_line(a)
    nb = discrete_header_line(b)
    na != nb && push!(out, "  header: $na -> $nb")
    attrs_semantically_equal(a, b) || push!(out, "  attrs: changed")
    for fname in FIELD_ORDER[a.kind]
        field_semantically_equal(a.kind, fname, a.fields, b.fields) && continue
        sa = node_field_snap(a.kind, a.fields, fname)
        sb = node_field_snap(b.kind, b.fields, fname)
        push!(out, string("  $fname: ", repr(sa), " -> ", repr(sb)))
    end
    out
end

function lock_structural_lines(ref::State, wt::State)::Vector{String}
    out = String[]
    for kind in NODE_KINDS
        ids_ref = Set(n.id for n in listnodes(ref, kind))
        ids_wt = Set(n.id for n in listnodes(wt, kind))
        added = sort(collect(setdiff(ids_wt, ids_ref)))
        removed = sort(collect(setdiff(ids_ref, ids_wt)))
        common = sort(collect(intersect(ids_ref, ids_wt)))
        if isempty(added) && isempty(removed) &&
           all(cid -> begin
                   nr = ref.nodes[cid]
                   nw = wt.nodes[cid]
                   nr.kind === nw.kind && node_semantically_equal(nr, nw)
               end,
               common)
            continue
        end
        push!(out, "## $(uppercase(String(kind)))")
        if !isempty(added)
            push!(out, "### added (+)")
            for id in added
                n = wt.nodes[id]
                ttl = isempty(n.title) ? "(no title)" : n.title
                push!(out, "+ $(discrete_header_line(n))  $ttl")
            end
        end
        if !isempty(removed)
            push!(out, "### removed (-)")
            for id in removed
                n = ref.nodes[id]
                ttl = isempty(n.title) ? "(no title)" : n.title
                push!(out, "- $(discrete_header_line(n))  $ttl")
            end
        end
        chlines = Tuple{Node,Node}[]
        for id in common
            nr = ref.nodes[id]
            nw = wt.nodes[id]
            (nr.kind === nw.kind && node_semantically_equal(nr, nw)) && continue
            push!(chlines, (nr, nw))
        end
        if !isempty(chlines)
            push!(out, "### changed (~)")
            for (nr, nw) in chlines
                push!(out, "~ $(nw.id)")
                append!(out, describe_node_changes(nr, nw))
            end
        end
        push!(out, "")
    end
    ere = multiset_diff(edge_multiset_counts(ref), edge_multiset_counts(wt))
    if !isempty(ere)
        push!(out, "## EDGES")
        for (sig, tup) in ere
            f, lbl, t = tup
            sig < 0 ? push!(out, "- e $f $lbl $t") : push!(out, "+ e $f $lbl $t")
        end
        push!(out, "")
    end
    if isempty(out)
        push!(out, "(no semantic changes)", "")
    end
    out
end

"""Structured diff payload for `--json`; keys mirror `print_lock_structural_diff` semantics."""
function lock_structural_diff_payload(ref::State, wt::State)::Dict{String,Any}
    nodes_payload = Dict{String,Any}()
    for kind in NODE_KINDS
        ids_ref = Set(n.id for n in listnodes(ref, kind))
        ids_wt = Set(n.id for n in listnodes(wt, kind))
        added_ids = sort!(collect(setdiff(ids_wt, ids_ref)))
        removed_ids = sort!(collect(setdiff(ids_ref, ids_wt)))
        common = sort!(collect(intersect(ids_ref, ids_wt)))
        chlines = Tuple{Node,Node}[]
        for id in common
            nr = ref.nodes[id]
            nw = wt.nodes[id]
            (nr.kind === nw.kind && node_semantically_equal(nr, nw)) && continue
            push!(chlines, (nr, nw))
        end
        if isempty(added_ids) && isempty(removed_ids) && isempty(chlines)
            continue
        end
        block = Dict{String,Any}()
        block["added"] = [
            Dict{String,String}(
                "id" => wt.nodes[id].id,
                "header" => discrete_header_line(wt.nodes[id]),
                "title" => isempty(wt.nodes[id].title) ? "(no title)" : wt.nodes[id].title,
            ) for id in added_ids
        ]
        block["removed"] = [
            Dict{String,String}(
                "id" => ref.nodes[id].id,
                "header" => discrete_header_line(ref.nodes[id]),
                "title" => isempty(ref.nodes[id].title) ? "(no title)" : ref.nodes[id].title,
            ) for id in removed_ids
        ]
        block["changed"] = [
            Dict{String,Any}(
                "id" => nw.id,
                "detail_lines" => describe_node_changes(nr, nw),
            ) for (nr, nw) in chlines
        ]
        nodes_payload[String(kind)] = block
    end
    ere = multiset_diff(edge_multiset_counts(ref), edge_multiset_counts(wt))
    edges_added = Dict{String,String}[]
    edges_removed = Dict{String,String}[]
    for (sig, tup) in ere
        f, lbl, t = tup
        d = Dict("from" => f, "label" => string(lbl), "to" => t)
        if sig < 0
            push!(edges_removed, d)
        else
            push!(edges_added, d)
        end
    end
    sem = !isempty(nodes_payload) || !isempty(edges_added) || !isempty(edges_removed)
    Dict{String,Any}(
        "nodes" => nodes_payload,
        "edges" => Dict{String,Any}("added" => edges_added, "removed" => edges_removed),
        "semantic_change" => sem,
    )
end

function print_lock_structural_diff(io::IO, ref_label::AbstractString,
    ref::State, wt::State)
    println(io, "# grove diff (ref -> worktree)")
    println(io, "")
    println(io, "baseline: `", ref_label, "`")
    println(io, "")
    for line in lock_structural_lines(ref, wt)
        println(io, line)
    end
    nothing
end

function git_repository_root(root::AbstractString)::Bool
    success(pipeline(`git -C $(abspath(String(root))) rev-parse --git-dir`;
        stdout=devnull, stderr=devnull))
end

"""`git show <ref>:gitpath`; returns `(nothing, err)` if the command fails."""
function git_show_path(root::AbstractString, ref::AbstractString, gitpath::AbstractString)::Tuple{Union{Nothing,String},String}
    root = abspath(String(root))
    spec = "$(ref):$(gitpath)"
    err = IOBuffer()
    wt = withenv("GIT_TERMINAL_PROMPT" => "0") do
        try
            read(pipeline(`git -C $(root) --no-pager show $(spec)`; stderr=err), String)
        catch
            nothing
        end
    end
    if wt === nothing
        estr = strip(String(take!(err)))
        isempty(estr) && return nothing, "git show failed"
        return nothing, estr
    end
    replace(wt, "\r\n" => "\n"), ""
end

function read_worktree_lock_text(path::AbstractString)::String
    isfile(path) || error("lock not found: $path")
    replace(read(path, String), "\r\n" => "\n")
end
