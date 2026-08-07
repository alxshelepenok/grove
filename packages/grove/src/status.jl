function work_significant_when_done(st::State, w::Node)::Bool
    w.kind !== :w && return false
    w.status !== :done && return false
    for did in implements_of(st, w)
        d = get(st.nodes, did, nothing)
        d !== nothing && d.kind === :d && d.status === :accepted && return true
    end
    w.type === :refactor && return true
    if w.type === :spike
        (w.cynefin !== nothing && w.cynefin === :complex) && return true
        return any(ed -> ed.label === :produces && ed.from == w.id, st.edges)
    end
    false
end

"""Human-readable alignment trigger lines (`protocol.md` §2.5). Empty if none apply."""
function alignment_triggers(st::State)::Vector{String}
    out = String[]
    for q in listnodes(st, :q)
        q.cynefin !== :chaotic && continue
        push!(out, "chaotic cynefin: $(q.id) ($(q.title)) status=$(q.status)")
    end
    for b in listnodes(st, :b)
        b.status !== :invalidated_blocking && continue
        push!(out, "blocked assumption: $(b.id) ($(b.title)) invalidated_blocking")
    end
    for w in listnodes(st, :w)
        w.status !== :done && continue
        work_significant_when_done(st, w) || continue
        push!(out,
            "significant done work: $(w.id) ($(w.title)) type=$(w.type) cynefin=$(w.cynefin)")
    end
    for g in listnodes(st, :g)
        g.status !== :verified && continue
        push!(out, "verified goal: $(g.id) ($(g.title))")
    end
    rs = ready(st)
    if isempty(rs)
        has_open_gap = false
        for q in listnodes(st, :q)
            q.status === :open && (has_open_gap = true)
        end
        for b in listnodes(st, :b)
            b.status in (:proposed, :testing) && (has_open_gap = true)
        end
        if has_open_gap
            push!(out, "idle: no ready work but open question(s) or active assumption benchmarking exist")
        end
    end
    sort!(out)
    out
end
