"""
Goal archive: mark `g` and every `w`/`d`/`q`/`b`/`a` that is *exclusive* to that goal
(refs(n) == {G}) and **connected** to `G` in the induced affinity graph.
"""

function _merge_goal_refs!(refs::Dict{String,Set{String}}, to::AbstractString, fr::AbstractString,
                           changed::Ref{Bool})::Nothing
    o = length(refs[to])
    union!(refs[to], refs[fr])
    length(refs[to]) != o && (changed[] = true)
    nothing
end

"""Propagate which goal IDs *reference* each node via `goals` fields and graph edges."""
function goal_reference_sets(st::State)::Dict{String,Set{String}}
    refs = Dict{String,Set{String}}()
    for id in keys(st.nodes)
        refs[id] = Set{String}()
    end
    for g in listnodes(st, :g; include_archived=true)
        push!(refs[g.id], g.id)
    end
    for w in listnodes(st, :w; include_archived=true)
        for gg in get(w.fields, :goals, String[])
            push!(refs[w.id], String(gg))
        end
    end
    ch = Ref(true)
    while ch[]
        ch[] = false
        for e in st.edges
            fk = get(st.nodes, e.from, nothing)
            tk = get(st.nodes, e.to, nothing)
            (fk === nothing || tk === nothing) && continue
            if e.label === :implements && fk.kind === :w && tk.kind === :d
                _merge_goal_refs!(refs, e.to, e.from, ch)
            elseif e.label === :produces && fk.kind === :w
                tk.kind in (:d, :q, :b) && _merge_goal_refs!(refs, e.to, e.from, ch)
            elseif e.label === :asks && fk.kind === :q && tk.kind === :w
                _merge_goal_refs!(refs, e.from, e.to, ch)
            elseif e.label === :tests && fk.kind === :b && tk.kind === :q
                _merge_goal_refs!(refs, e.from, e.to, ch)
            elseif e.label === :targets && fk.kind === :b && tk.kind === :w
                _merge_goal_refs!(refs, e.from, e.to, ch)
            elseif e.label === :causes && fk.kind === :t && tk.kind === :w
                _merge_goal_refs!(refs, e.from, e.to, ch)
            elseif e.label === :supersedes && fk.kind === :d && tk.kind === :d
                _merge_goal_refs!(refs, e.from, e.to, ch)
                _merge_goal_refs!(refs, e.to, e.from, ch)
            end
        end
        for w in listnodes(st, :w; include_archived=true)
            tid = strip(string(get(w.fields, :theme, "")))
            isempty(tid) && continue
            haskey(st.nodes, tid) || continue
            o = length(refs[tid])
            union!(refs[tid], refs[w.id])
            length(refs[tid]) != o && (ch[] = true)
        end
    end
    refs
end

function _exclusive_want(st::State, refs::Dict{String,Set{String}}, gid::String)::Set{String}
    want = Set{String}()
    gset = Set([gid])
    for (id, rs) in refs
        haskey(st.nodes, id) || continue
        st.nodes[id].archived && continue
        rs == gset || continue
        push!(want, id)
    end
    want
end

function _affinity_neighbors(st::State, u::String, want::Set{String}, gid::String)::Vector{String}
    out = Set{String}()
    if u == gid
        for w in listnodes(st, :w)
            (w.archived || w.id ∉ want) && continue
            gid in get(w.fields, :goals, String[]) || continue
            push!(out, w.id)
        end
    end
    if haskey(st.nodes, u) && st.nodes[u].kind === :w
        w = st.nodes[u]
        !w.archived && gid in get(w.fields, :goals, String[]) && gid in want && push!(out, gid)
    end
    for e in st.edges
        other = nothing
        if e.from == u
            other = e.to
        elseif e.to == u
            other = e.from
        else
            continue
        end
        other in want || continue
        push!(out, other)
    end
    collect(out)
end

"""IDS to set `.archived` when archiving verified goal `gid`."""
function exclusive_archive_ids(st::State, gid::String)::Set{String}
    refs = goal_reference_sets(st)
    want = _exclusive_want(st, refs, gid)
    gid in want || return Set{String}()
    seen = Set{String}()
    stack = String[gid]
    while !isempty(stack)
        u = pop!(stack)
        u in seen && continue
        u in want || continue
        push!(seen, u)
        for v in _affinity_neighbors(st, u, want, gid)
            v in seen || push!(stack, v)
        end
    end
    seen
end

function distill_linked_da_ids(st::State, mass::Set{String})::Vector{String}
    out = Set{String}()
    for e in st.edges
        xid = nothing
        if e.label === :produces && e.from in mass
            xid = e.to
        elseif e.label === :distills && e.to in mass
            xid = e.from
        end
        xid === nothing && continue
        n = get(st.nodes, xid, nothing)
        n === nothing && continue
        n.kind === :y || continue
        n.status === :active || continue
        push!(out, n.id)
    end
    sort!(collect(out))
end
