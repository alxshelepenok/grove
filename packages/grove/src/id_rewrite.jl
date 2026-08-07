"""True if `id` occurs in line as its own contiguous token run (hyphen alphanumeric)."""
function id_occurs_exactly_as_token(line::AbstractString, id::AbstractString)::Bool
    id0 = String(id)
    isempty(id0) && return false
    li = String(line)

    function isidc(c::Char)::Bool
        ('A' <= c <= 'Z') || ('a' <= c <= 'z') || ('0' <= c <= '9') || c == '-'
    end

    i = firstindex(li)
    while i <= lastindex(li)
        r = findnext(id0, li, i)
        r === nothing && return false
        a = first(r)
        b = last(r)
        left_ok = a == firstindex(li) || !isidc(li[prevind(li, a)])
        right_ok = b == lastindex(li) || !isidc(li[nextind(li, b)])
        (left_ok && right_ok) && return true
        i = nextind(li, b)
    end
    false
end

function renumber_blocked_by_done_evidence(st::State, old_id::AbstractString)::Bool
    old0 = String(old_id)
    for w in listnodes(st, :w)
        w.status === :done || continue
        for line in get(w.fields, :evidence, String[])
            id_occurs_exactly_as_token(line, old0) && return true
        end
    end
    false
end

function _replace_reflist!(v::AbstractVector, old_id::AbstractString, new_id::AbstractString)::Nothing
    o, nw = String(old_id), String(new_id)
    for i in eachindex(v)
        string(v[i]) == o || continue
        v[i] = nw
    end
    nothing
end

function _rewrite_node_fields_after_renumber!(n::Node, old_id::AbstractString, new_id::AbstractString)::Nothing
    o, nw = String(old_id), String(new_id)
    for (k, frm) in FIELD_CATALOG
        k[1] === n.kind || continue
        fld = k[2]
        if frm === :reflist && haskey(n.fields, fld)
            v = n.fields[fld]
            v isa AbstractVector || continue
            _replace_reflist!(v, o, nw)
        elseif frm === :fitness && haskey(n.fields, fld)
            d = n.fields[fld]
            d isa AbstractDict || continue
            if haskey(d, o)
                d[nw] = d[o]
                delete!(d, o)
            end
        elseif frm === :single && haskey(n.fields, fld)
            val = n.fields[fld]
            val isa AbstractString || continue
            string(val) == o && (n.fields[fld] = nw)
        end
    end
    nothing
end

"""
Rewrite `old_id → new_id` everywhere in graph structure (`nodes`, `edges`, structured fields and attrs).

Caller must enforce same family prefix, uniqueness of `new_id`, evidence guard on `done` W records, etc.
"""
function apply_renumber!(st::State, old_id::AbstractString, new_id::AbstractString)::Nothing
    o = String(old_id)
    nw = String(new_id)
    o == nw && return nothing
    haskey(st.nodes, o) || error("rename: missing record $o")
    haskey(st.nodes, nw) && error("rename: target already exists $nw")
    p0, _ = parse_id_numeric(o)
    p1, _ = parse_id_numeric(nw)
    p0 == p1 || error("rename: family mismatch $o vs $nw")

    n = st.nodes[o]
    delete!(st.nodes, o)
    n.id = nw
    st.nodes[nw] = n

    for e in st.edges
        e.from == o && (e.from = nw)
        e.to == o && (e.to = nw)
    end

    for x in values(st.nodes)
        if haskey(x.attrs, "goal") && string(x.attrs["goal"]) == o
            x.attrs["goal"] = nw
        end
        _rewrite_node_fields_after_renumber!(x, o, nw)
    end

    journal_reconcile_counters!(st)
    stamp_touch_node!(n)
    nothing
end
