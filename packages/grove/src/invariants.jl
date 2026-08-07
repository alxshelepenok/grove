const WIP_LIMIT_DEFAULT = 2

function i1_dor_on_progress(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        w.status === :progress || continue
        dor(st, w; pin_coverage=true) || push!(out, "I1: $(w.id) is `progress` but DoR ≢ ⊤")
    end
    out
end

function i2_spike_outputs(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        (w.type === :spike && w.status === :done) || continue
        has_any = any(e -> e.label === :produces && e.from == w.id, st.edges)
        has_any || push!(out,
            "I2: $(w.id) is a done spike but `produces` is empty (no outgoing `produces` edges)")
    end
    out
end

function i3_done_has_evidence(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        w.status === :done || continue
        ev = get(w.fields, :evidence, String[])
        isempty(ev) && push!(out, "I3: $(w.id) is `done` but `evidence` is empty")
    end
    out
end

function i4_wip_limit(st::State; limit::Int=WIP_LIMIT_DEFAULT)::Vector{String}
    n = count(w -> w.status === :progress, listnodes(st, :w))
    n > limit ? ["I4: WIP $(n) exceeds limit $(limit)"] : String[]
end

function i5_blocks_terminal(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        w.status === :progress || continue
        for p in blocked_by(st, w.id)
            np = get(st.nodes, p, nothing)
            if np === nothing
                push!(out, "I5: $(w.id) blocked by missing $p"); continue
            end
            clears_blocks_predecessor(np) ||
                push!(out, "I5: $(w.id) is `progress` but blocker $(p) ($(np.status)) does not satisfy blocks clearance (goals must be verified)")
        end
    end
    out
end

function i7_blocks_dag(st::State)::Vector{String}
    succ = Dict{String,Vector{String}}()
    nodes = Set{String}()
    for e in st.edges
        e.label === :blocks || continue
        push!(get!(succ, e.from, String[]), e.to)
        push!(nodes, e.from); push!(nodes, e.to)
    end
    indeg = Dict{String,Int}(id => 0 for id in nodes)
    for (_, vs) in succ, v in vs
        indeg[v] = get(indeg, v, 0) + 1
    end
    q = [id for (id, d) in indeg if d == 0]
    visited = 0
    while !isempty(q)
        x = pop!(q)
        visited += 1
        for s in get(succ, x, String[])
            indeg[s] -= 1
            indeg[s] == 0 && push!(q, s)
        end
    end
    visited == length(nodes) ? String[] : ["I7: blocks graph contains a cycle"]
end

function i9_feature_bchain(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        (w.type === :feature && w.status in (:ready, :progress)) || continue
        for b in bchain(st, w)
            n = get(st.nodes, b, nothing)
            n === nothing && continue
            n.status in (:validated, :invalidated_acceptable) ||
                push!(out, "I9: $(w.id) is `$(w.status)` but $(b) is `$(n.status)`")
        end
    end
    out
end

function i10_done_fitness(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        w.status === :done || continue
        gs = get(w.fields, :goals, String[])
        f = get(w.fields, :fitness, Dict{String,Int}())
        for g in gs
            haskey(f, g) || push!(out, "I10: $(w.id) is `done` but no fitness delta for $g")
        end
    end
    out
end

function i11_progress_has_session_claim(st::State)::Vector{String}
    out = String[]
    for w in listnodes(st, :w)
        w.status === :progress || continue
        progress_has_session_record(w) ||
            push!(out, "I11: $(w.id) is `progress` but has no session token")
    end
    out
end

function check_orphan_edges(st::State)::Vector{String}
    out = String[]
    for e in st.edges
        haskey(st.nodes, e.from) || push!(out, "edge endpoint missing: $(e.from)")
        haskey(st.nodes, e.to) || push!(out, "edge endpoint missing: $(e.to)")
    end
    out
end

function check_edge_types(st::State)::Vector{String}
    out = String[]
    for e in st.edges
        from = get(st.nodes, e.from, nothing)
        to = get(st.nodes, e.to, nothing)
        (from === nothing || to === nothing) && continue
        ok = if e.label === :blocks
            to.kind === :w
        elseif e.label === :causes
            from.kind === :t && to.kind === :w
        elseif e.label === :implements
            from.kind === :w && to.kind === :d
        elseif e.label === :asks
            from.kind === :q
        elseif e.label === :tests
            from.kind === :b && to.kind === :q
        elseif e.label === :targets
            from.kind === :b && to.kind === :w
        elseif e.label === :produces
            from.kind === :w && to.kind in (:d, :q, :b, :y)
        elseif e.label === :supersedes
            (from.kind === :d && to.kind === :d) || (from.kind === :y && to.kind === :y)
        elseif e.label === :distills
            from.kind === :y && to.kind in (:d, :q, :b)
        else
            false
        end
        ok || push!(out, "edge type mismatch: $(e.from) -$(e.label)-> $(e.to)")
    end
    out
end

function discovery_anchor_issues(st::State, x::Node)::Vector{String}
    out = String[]
    x.kind === :y || return out
    x.archived && return out
    has_origin = false
    for e in st.edges
        if e.label === :produces && e.to == x.id
            src = get(st.nodes, e.from, nothing)
            src !== nothing && src.kind === :w && (has_origin = true)
        elseif e.label === :distills && e.from == x.id
            dst = get(st.nodes, e.to, nothing)
            dst !== nothing && dst.kind in (:d, :q, :b) && (has_origin = true)
        end
        has_origin && break
    end
    has_origin || push!(out,
        "I12: $(x.id) has no provenance edge (needs `produces` from a W or `distills` to a D/Q/B)")
    surface = get(x.fields, :surface, String[])
    why = get(x.fields, :why, String[])
    (isempty(surface) && !prose_field_nonempty(why)) && push!(out,
        "I12: $(x.id) has empty `surface` and empty `why` (≥1 anchor required)")
    tags = get(x.fields, :tags, String[])
    isempty(tags) && push!(out, "I12: $(x.id) has empty `tags` (≥1 glossary term required)")
    out
end

function check_discovery_anchors(st::State)::Vector{String}
    out = String[]
    for x in listnodes(st, :y)
        append!(out, discovery_anchor_issues(st, x))
    end
    out
end

function check_area_membership(st::State)::Vector{String}
    out = String[]
    for g in listnodes(st, :g; include_archived=true)
        aref = get(g.fields, :area, "")
        if !(aref isa AbstractString) || isempty(strip(String(aref)))
            push!(out, "I13: $(g.id) has no `area` field (every goal belongs to an area: `grove set $(g.id) area=A-NN`)")
            continue
        end
        zn = get(st.nodes, String(aref), nothing)
        (zn === nothing || zn.kind !== :a) &&
            push!(out, "I13: $(g.id) area $aref does not reference an existing area (a) node")
    end
    out
end

function check_all(st::State)::Vector{String}
    vcat(
        i1_dor_on_progress(st),
        i2_spike_outputs(st),
        i3_done_has_evidence(st),
        i4_wip_limit(st),
        i5_blocks_terminal(st),
        i7_blocks_dag(st),
        i9_feature_bchain(st),
        i10_done_fitness(st),
        i11_progress_has_session_claim(st),
        check_orphan_edges(st),
        check_edge_types(st),
        check_discovery_anchors(st),
        check_area_membership(st),
    )
end

"""Return `nothing` on success, otherwise an error string (edge not added)."""
function validate_and_push_edge!(
    st::State, from::AbstractString, label::Symbol, to::AbstractString;
    bump_nodes::Bool=true,
)::Union{Nothing,String}
    from = String(strip(from))
    to = String(strip(to))
    label in EDGE_LABELS || return "unknown edge label: $(label)"
    haskey(st.nodes, from) || return "missing node $(from)"
    haskey(st.nodes, to) || return "missing node $(to)"
    if any(e -> e.from == from && e.label === label && e.to == to, st.edges)
        return nothing
    end
    e = Edge(from, label, to)
    push!(st.edges, e)
    stamp_new_edge!(e)
    if label === :blocks && !isempty(i7_blocks_dag(st))
        pop!(st.edges)
        return "I7: blocks introduces a cycle"
    end
    et = check_edge_types(st)
    if !isempty(et)
        pop!(st.edges)
        return et[end]
    end
    if bump_nodes
        stamp_touch_node!(getnode(st, from))
        stamp_touch_node!(getnode(st, to))
    end
    nothing
end

