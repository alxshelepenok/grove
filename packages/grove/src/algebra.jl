function blocked_by(st::State, id::AbstractString)::Vector{String}
    [e.from for e in st.edges if e.label === :blocks && e.to == id]
end

function blocks_of(st::State, id::AbstractString)::Vector{String}
    [e.to for e in st.edges if e.label === :blocks && e.from == id]
end

function deps(st::State, id::AbstractString)::Vector{String}
    seen = Set{String}()
    order = String[]
    function visit(x)
        for p in blocked_by(st, x)
            if !(p in seen)
                push!(seen, p)
                visit(p)
                push!(order, p)
            end
        end
    end
    visit(String(id))
    order
end

function impact(st::State, id::AbstractString)::Vector{String}
    seen = Set{String}()
    order = String[]
    function visit(x)
        for s in blocks_of(st, x)
            if !(s in seen)
                push!(seen, s)
                push!(order, s)
                visit(s)
            end
        end
    end
    visit(String(id))
    order
end

function preds_clear(st::State, id::AbstractString)::Bool
    for p in blocked_by(st, id)
        n = get(st.nodes, p, nothing)
        n === nothing && return false
        clears_blocks_predecessor(n) || return false
    end
    true
end

ac_of(n::Node) = get(n.fields, :ac, String[])
goals_of(n::Node) = get(n.fields, :goals, String[])

"""True if prose field has at least one non-empty (after strip) line."""
function prose_field_nonempty(lines)::Bool
    for s in lines
        !isempty(strip(string(s))) && return true
    end
    return false
end

"""Refactor DoR: some non-archived T has (T, causes, w)."""
function refactor_materialised_root_cause(st::State, w::Node)::Tuple{Bool,String}
    parts = String[]
    for e in st.edges
        e.label === :causes || continue
        e.to != w.id && continue
        a = get(st.nodes, e.from, nothing)
        a === nothing && continue
        a.kind !== :t && continue
        a.archived && continue
        push!(parts, e.from)
    end
    isempty(parts) && return (false, "")
    return (true, join(sort!(unique(parts)), ", "))
end

function asks_of(st::State, w::Node)::Vector{String}
    [e.from for e in st.edges if e.label === :asks && e.to == w.id]
end

function implements_of(st::State, w::Node)::Vector{String}
    [e.to for e in st.edges if e.label === :implements && e.from == w.id]
end

function bchain(st::State, w::Node)::Vector{String}
    out = Set{String}()
    for e in st.edges
        e.label === :targets || continue
        e.to == w.id || continue
        fromn = get(st.nodes, e.from, nothing)
        fromn !== nothing && fromn.kind === :b && push!(out, e.from)
    end
    for e in st.edges
        e.label === :tests || continue
        bf = get(st.nodes, e.from, nothing)
        qt = get(st.nodes, e.to, nothing)
        (bf === nothing || qt === nothing) && continue
        (bf.kind === :b && qt.kind === :q) || continue
        if any(ed -> ed.label === :asks && ed.from == e.to && ed.to == w.id, st.edges)
            push!(out, e.from)
        end
    end
    sort!(collect(out))
end

"""Re-derive artifact `status` from themed work items (I₆)."""
function rederive_artifacts!(st::State)
    for a in listnodes(st, :t)
        prev = a.status
        ws = Node[w for w in listnodes(st, :w) if get(w.fields, :theme, "") == a.id]
        if isempty(ws)
            a.status = :open
        else
            a.status = all(w -> isterminal(:w, w.status), ws) ? :done : :open
        end
        a.status !== prev && stamp_touch_node!(a)
    end
    nothing
end

function active_discovery_surfaces(st::State)::Set{String}
    out = Set{String}()
    for x in listnodes(st, :y)
        x.status === :active || continue
        union!(out, String.(get(x.fields, :surface, String[])))
    end
    out
end

function coverage(st::State, w::Node)::Tuple{Float64,Vector{String},Vector{String}}
    surface_w = String.(get(w.fields, :surface, String[]))
    isempty(surface_w) && return (0.0, String[], String[])
    act = active_discovery_surfaces(st)
    covered = sort([p for p in surface_w if p in act])
    uncovered = sort([p for p in surface_w if !(p in act)])
    (length(covered) / length(surface_w), covered, uncovered)
end

function triage_rows(st::State)::Vector{NamedTuple{(:w,:title,:cov,:declared,:uncertainty,:fragile,:suggestion),Tuple{String,String,Float64,Bool,Int,Bool,String}}}
    rows = NamedTuple{(:w,:title,:cov,:declared,:uncertainty,:fragile,:suggestion),Tuple{String,String,Float64,Bool,Int,Bool,String}}[]
    for w in listnodes(st, :w)
        isterminal(:w, w.status) && continue
        declared = !isempty(get(w.fields, :surface, String[]))
        cov, _, _ = coverage(st, w)
        χ = 0
        for q in asks_of(st, w)
            n = get(st.nodes, q, nothing)
            n !== nothing && n.status === :open && (χ += 1)
        end
        for b in bchain(st, w)
            n = get(st.nodes, b, nothing)
            (n === nothing || !(n.status in (:validated, :invalidated_acceptable))) && (χ += 1)
        end
        χ += count(t -> !t[2], dor_breakdown(st, w))
        fragile = any(t -> t[2] <= 1, goal_fragility(st, w))
        suggestion = if !declared
            "declare surface"
        elseif cov == 0.0
            "spike to create coverage"
        elseif χ > 0
            "resolve open Q/B and DoR gaps"
        elseif fragile
            "add a redundant path (blocks)"
        elseif cov < 0.5
            "deepen coverage"
        else
            "ready to deliver"
        end
        push!(rows, (w=w.id, title=w.title, cov=cov, declared=declared,
            uncertainty=χ, fragile=fragile, suggestion=suggestion))
    end
    sort!(rows; by=r -> (r.cov, -r.uncertainty, r.w))
    rows
end

function parse_requires_coverage(v)::Union{Nothing,Float64}
    v === nothing && return nothing
    s = strip(string(v))
    s == "true" && return 0.5
    x = tryparse(Float64, s)
    x === nothing && return nothing
    0.0 < x <= 1.0 || return nothing
    x
end

function coverage_requirement(st::State, w::Node)::Union{Nothing,Float64}
    θ = nothing
    for gid in goals_of(w)
        g = get(st.nodes, gid, nothing)
        g === nothing && continue
        v = parse_requires_coverage(get(g.attrs, "requires_coverage", nothing))
        v === nothing && continue
        θ = θ === nothing ? v : max(θ, v)
    end
    tid = get(w.fields, :theme, "")
    if !isempty(tid)
        a = get(st.nodes, tid, nothing)
        if a !== nothing
            v = parse_requires_coverage(get(a.attrs, "requires_coverage", nothing))
            v !== nothing && (θ = θ === nothing ? v : max(θ, v))
        end
    end
    θ
end

function dor_breakdown(st::State, w::Node; pin_coverage::Bool=false)::Vector{Tuple{String,Bool,String}}
    out = Tuple{String,Bool,String}[]
    g = goals_of(w)
    push!(out, ("goals(w) ≠ ∅", !isempty(g), join(g, ", ")))
    ac = ac_of(w)
    push!(out, ("AC(w) ≠ ∅", !isempty(ac), string(length(ac), " entries")))
    asks = asks_of(st, w)
    asks_ok = all(q -> begin
            n = get(st.nodes, q, nothing)
            n !== nothing && isterminal(:q, n.status)
        end, asks)
    push!(out, ("∀ q ∈ asks(w), q terminal", asks_ok, join(asks, ", ")))
    if w.type === :feature
        chain = bchain(st, w)
        chain_ok = all(b -> begin
                n = get(st.nodes, b, nothing)
                n !== nothing && n.status in (:validated, :invalidated_acceptable)
            end, chain)
        push!(out, ("BChain validated", chain_ok, join(chain, ", ")))
    else
        push!(out, ("BChain validated", true, "(non-feature)"))
    end
    fitness = get(w.fields, :fitness, Dict{String,Int}())
    fit_ok = !isempty(g) && all(gid -> haskey(fitness, gid), g)
    push!(out, ("fitness deltas set ∀ g", fit_ok,
        join([string(k, "=", v >= 0 ? "+" : "", v) for (k, v) in fitness], ", ")))
    es = get(w.fields, :evidence_strategy, String[])
    push!(out, ("evidence_strategy ≠ ∅", !isempty(es), string(length(es), " entries")))
    if w.type === :feature
        hyp = get(w.fields, :hypothesis, String[])
        push!(out, ("hypothesis ≠ ⊥", !isempty(hyp), ""))
    else
        push!(out, ("hypothesis ≠ ⊥", true, "(non-feature)"))
    end
    if w.type === :bug
        rp = get(w.fields, :repro, String[])
        r_ok = prose_field_nonempty(rp)
        push!(out, ("repro(w) ≠ ∅", r_ok, r_ok ? string(length(rp), " entries") : ""))
    else
        push!(out, ("repro(w) ≠ ∅", true, "(non-bug)"))
    end
    if w.type === :spike
        ex = get(w.fields, :exit, String[])
        e_ok = prose_field_nonempty(ex)
        push!(out, ("exit(w) ≠ ∅", e_ok, e_ok ? string(length(ex), " entries") : ""))
    else
        push!(out, ("exit(w) ≠ ∅", true, "(non-spike)"))
    end
    if w.type === :refactor
        rc_ok, rc_detail = refactor_materialised_root_cause(st, w)
        push!(out, ("(A, causes, w) via materialised A", rc_ok, rc_detail))
    else
        push!(out, ("(A, causes, w) via materialised A", true, "(non-refactor)"))
    end
    push!(out, ("cynefin ≠ chaotic", w.cynefin !== :chaotic,
        w.cynefin === nothing ? "" : String(w.cynefin)))
    θ = coverage_requirement(st, w)
    label = "coverage(w) ≥ θ"
    if θ === nothing
        push!(out, (label, true, "(coverage not required)"))
    elseif !(w.type === :feature && w.cynefin === :complex)
        push!(out, (label, true, "(non-complex-feature)"))
    elseif pin_coverage
        push!(out, (label, true, "(pinned at transition)"))
    else
        surface_w = get(w.fields, :surface, String[])
        ratio, _, uncovered = coverage(st, w)
        θs = @sprintf("%.2f", θ)
        if isempty(surface_w)
            push!(out, (label, false,
                "no declared surface; declare via field $(w.id) surface add …"))
        elseif ratio < θ
            shown = first(uncovered, 5)
            det = string(@sprintf("%.2f", ratio), " < ", θs, "; uncovered: ", join(shown, ", "))
            length(uncovered) > 5 && (det *= " … (+$(length(uncovered) - 5) more)")
            push!(out, (label, false, det))
        else
            push!(out, (label, true, string(@sprintf("%.2f", ratio), " ≥ ", θs)))
        end
    end
    out
end

dor(st::State, w::Node; pin_coverage::Bool=false)::Bool =
    all(t -> t[2], dor_breakdown(st, w; pin_coverage=pin_coverage))
dor(st::State, id::AbstractString)::Bool = dor(st, getnode(st, id))

function ready(st::State)::Vector{Node}
    out = Node[]
    for w in listnodes(st, :w)
        w.status === :ready || continue
        preds_clear(st, w.id) || continue
        dor(st, w) || continue
        push!(out, w)
    end
    out
end

function critical_path(st::State)::Vector{String}
    active = Set(w.id for w in listnodes(st, :w) if !isterminal(:w, w.status))
    succ = Dict{String,Vector{String}}()
    indeg = Dict{String,Int}(id => 0 for id in active)
    for e in st.edges
        e.label === :blocks || continue
        e.from in active || continue
        e.to in active || continue
        push!(get!(succ, e.from, String[]), e.to)
        indeg[e.to] = get(indeg, e.to, 0) + 1
    end
    queue = sort!([id for (id, d) in indeg if d == 0])
    topo = String[]
    indeg_w = copy(indeg)
    while !isempty(queue)
        x = popfirst!(queue)
        push!(topo, x)
        for s in get(succ, x, String[])
            indeg_w[s] -= 1
            if indeg_w[s] == 0
                push!(queue, s)
                sort!(queue)
            end
        end
    end
    dist = Dict{String,Int}(id => 1 for id in active)
    parent = Dict{String,Union{String,Nothing}}(id => nothing for id in active)
    for x in topo
        for s in get(succ, x, String[])
            if dist[x] + 1 > dist[s]
                dist[s] = dist[x] + 1
                parent[s] = x
            end
        end
    end
    isempty(dist) && return String[]
    tail = first(sort(collect(active); by=id -> (-dist[id], id)))
    chain = String[]
    cur::Union{String,Nothing} = tail
    while cur !== nothing
        push!(chain, cur)
        cur = parent[cur]
    end
    reverse(chain)
end

function packet(st::State, w::Node)::String
    io = IOBuffer()
    println(io, "# Execution packet: ", w.id, " (", w.title, ")")
    println(io)
    println(io, "type=", w.type, "  status=", w.status, "  cynefin=", w.cynefin)
    println(io)
    if !isempty(goals_of(w))
        println(io, "**Goals:** ", join(goals_of(w), ", "))
    end
    fitness = get(w.fields, :fitness, Dict{String,Int}())
    if !isempty(fitness)
        parts = [string(k, "=", v >= 0 ? "+" : "", v) for (k, v) in fitness]
        println(io, "**Fitness contribution:** ", join(parts, ", "))
    end
    println(io)
    for (label, fname) in (("Why", :why), ("Repro", :repro), ("Hypothesis", :hypothesis), ("Exit (spike)", :exit),
        ("Acceptance criteria", :ac),
        ("Evidence strategy", :evidence_strategy),
        ("Plan", :plan), ("Evidence", :evidence))
        lines = get(w.fields, fname, String[])
        isempty(lines) && continue
        println(io, "## ", label)
        println(io)
        for ln in lines
            println(io, "- ", ln)
        end
        println(io)
    end
    # Linked decisions.
    for did in implements_of(st, w)
        d = get(st.nodes, did, nothing)
        d === nothing && continue
        println(io, "## Decision ", d.id, ": ", d.title, "  (", d.status, ")")
        println(io)
        for fname in (:context, :options, :decision, :consequences, :validation)
            lines = get(d.fields, fname, String[])
            isempty(lines) && continue
            println(io, "**", String(fname), ":**")
            for ln in lines
                println(io, "- ", ln)
            end
            println(io)
        end
    end
    for bid in bchain(st, w)
        b = get(st.nodes, bid, nothing)
        b === nothing && continue
        println(io, "## Assumption ", b.id, ": ", b.title, "  (", b.status, ", ", b.cynefin, ")")
        for fname in (:vm, :threshold, :result)
            lines = get(b.fields, fname, String[])
            isempty(lines) && continue
            println(io, "**", String(fname), ":**")
            for ln in lines
                println(io, "- ", ln)
            end
        end
        println(io)
    end
    for qid in asks_of(st, w)
        q = get(st.nodes, qid, nothing)
        q === nothing && continue
        println(io, "## Question ", q.id, ": ", q.title, "  (", q.status, ", ", q.cynefin, ")")
        outcome = get(q.fields, :outcome, String[])
        if !isempty(outcome)
            println(io, "**outcome:**")
            for ln in outcome
                println(io, "- ", ln)
            end
        end
        println(io)
    end
    println(io, "## Definition of Ready")
    println(io)
    for (label, ok, detail) in dor_breakdown(st, w)
        sym = ok ? "⊤" : "⊥"
        if isempty(detail)
            println(io, "- ", sym, "  ", label, ".")
        else
            println(io, "- ", sym, "  ", label, " (", detail, ").")
        end
    end
    overall = dor(st, w) ? "⊤" : "⊥"
    println(io)
    println(io, "**result: ", overall, "**")
    String(take!(io))
end

function bounded_cone_walk(st::State, id::AbstractString, step::Function;
    depth::Int=4, maxcount::Int=50)::NamedTuple{(:ids,:truncated),Tuple{Vector{String},Bool}}
    seen = Set{String}([String(id)])
    ids = String[]
    truncated = false
    frontier = String[String(id)]
    hops = 0
    while hops < depth && !isempty(frontier) && !truncated
        hops += 1
        level = String[]
        for x in frontier, y in step(st, x)
            y in seen && continue
            push!(seen, y)
            push!(level, y)
        end
        sort!(level)
        room = max(maxcount - length(ids), 0)
        if length(level) > room
            append!(ids, level[1:room])
            truncated = true
        else
            append!(ids, level)
            frontier = level
        end
    end
    if !truncated && !isempty(frontier)
        truncated = any(x -> any(y -> !(y in seen), step(st, x)), frontier)
    end
    (ids=ids, truncated=truncated)
end

backward_cone(st::State, id::AbstractString; depth::Int=4, maxcount::Int=50) =
    bounded_cone_walk(st, id, blocked_by; depth=depth, maxcount=maxcount)

forward_cone(st::State, id::AbstractString; depth::Int=4, maxcount::Int=50) =
    bounded_cone_walk(st, id, blocks_of; depth=depth, maxcount=maxcount)

function contraction_order(st::State, ids)::Vector{String}
    keep = Set{String}(String(x) for x in ids)
    succ = Dict{String,Vector{String}}()
    indeg = Dict{String,Int}(id => 0 for id in keep)
    for e in st.edges
        e.label === :blocks || continue
        (e.from in keep && e.to in keep) || continue
        push!(get!(succ, e.from, String[]), e.to)
        indeg[e.to] = get(indeg, e.to, 0) + 1
    end
    queue = sort!([id for (id, d) in indeg if d == 0])
    order = String[]
    while !isempty(queue)
        x = popfirst!(queue)
        push!(order, x)
        for s in get(succ, x, String[])
            indeg[s] -= 1
            if indeg[s] == 0
                push!(queue, s)
                sort!(queue)
            end
        end
    end
    order
end

function node_connectivity(st::State, src::AbstractString, dst::AbstractString)::Int
    src = String(src)
    dst = String(dst)
    src == dst && return 0
    sn = get(st.nodes, src, nothing)
    dn = get(st.nodes, dst, nothing)
    (sn === nothing || dn === nothing) && return 0
    (sn.archived || dn.archived) && return 0
    ids = sort!([id for (id, n) in st.nodes if !n.archived])
    slot = Dict{String,Int}(id => i for (i, id) in enumerate(ids))
    n = 2 * length(ids)
    cap = zeros(Int, n, n)
    unbounded = length(ids) + 1
    for (id, i) in slot
        cap[2i-1, 2i] = (id == src || id == dst) ? unbounded : 1
    end
    for e in st.edges
        e.label === :blocks || continue
        u = get(slot, e.from, 0)
        v = get(slot, e.to, 0)
        (u == 0 || v == 0) && continue
        cap[2u, 2v-1] = unbounded
    end
    source = 2 * slot[src]
    sink = 2 * slot[dst] - 1
    flow = 0
    while true
        prev = zeros(Int, n)
        seen = falses(n)
        seen[source] = true
        queue = Int[source]
        while !isempty(queue) && !seen[sink]
            u = popfirst!(queue)
            for v in 1:n
                (cap[u, v] > 0 && !seen[v]) || continue
                seen[v] = true
                prev[v] = u
                push!(queue, v)
            end
        end
        seen[sink] || break
        add = unbounded
        v = sink
        while v != source
            u = prev[v]
            add = min(add, cap[u, v])
            v = u
        end
        v = sink
        while v != source
            u = prev[v]
            cap[u, v] -= add
            cap[v, u] += add
            v = u
        end
        flow += add
    end
    flow
end

function goal_fragility(st::State, w::Node)::Vector{Tuple{String,Int}}
    out = Tuple{String,Int}[]
    for g in sort!(unique(goals_of(w)))
        push!(out, (g, node_connectivity(st, g, w.id)))
    end
    out
end

function discovery_anchor_count(st::State, discovery::Node, surfaces::Set{String}, tags::Set{String}, cone::Set{String})::Int
    anchors = 0
    isempty(intersect!(Set{String}(get(discovery.fields, :surface, String[])), surfaces)) || (anchors += 1)
    isempty(intersect!(Set{String}(get(discovery.fields, :tags, String[])), tags)) || (anchors += 1)
    linked = any(e -> (e.from == discovery.id && e.to in cone) || (e.to == discovery.id && e.from in cone), st.edges)
    linked && (anchors += 1)
    anchors
end

discovery_anchor_matches(st::State, discovery::Node, surfaces::Set{String}, tags::Set{String}, cone::Set{String})::Bool =
    discovery_anchor_count(st, discovery, surfaces, tags, cone) > 0

function relevant_discoveries(st::State, w::Node, cone_ids; maxcount::Int=50)::Vector{String}
    cone = Set{String}(String(x) for x in cone_ids)
    w_surface = Set{String}(get(w.fields, :surface, String[]))
    cone_tags = Set{String}(get(w.fields, :tags, String[]))
    for id in cone
        n = get(st.nodes, id, nothing)
        n === nothing && continue
        union!(cone_tags, get(n.fields, :tags, String[]))
    end
    scored = Tuple{Int,String}[]
    for discovery in listnodes(st, :y)
        discovery.status === :active || continue
        anchors = discovery_anchor_count(st, discovery, w_surface, cone_tags, cone)
        anchors > 0 && push!(scored, (-anchors, discovery.id))
    end
    out = String[]
    for (_, id) in sort!(scored)
        push!(out, id)
    end
    first(out, maxcount)
end

area_goals(st::State, z::Node)::Vector{Node} =
    [g for g in listnodes(st, :g) if get(g.fields, :area, "") == z.id]

function area_work(st::State, z::Node)::Vector{Node}
    gids = Set{String}(g.id for g in area_goals(st, z))
    [w for w in listnodes(st, :w) if any(g -> g in gids, goals_of(w))]
end

function area_surface(st::State, z::Node)::Set{String}
    out = Set{String}(String.(get(z.fields, :surface, String[])))
    for w in area_work(st, z)
        union!(out, String.(get(w.fields, :surface, String[])))
    end
    out
end

function area_tags(st::State, z::Node)::Set{String}
    out = Set{String}()
    for n in vcat(area_goals(st, z), area_work(st, z))
        union!(out, get(n.fields, :tags, String[]))
    end
    out
end

function area_node_ids(st::State, z::Node)::Set{String}
    wids = Set{String}(w.id for w in area_work(st, z))
    out = union!(Set{String}(g.id for g in area_goals(st, z)), wids)
    for n in values(st.nodes)
        n.archived && continue
        n.kind in (:q, :b, :d) || continue
        linked = any(e -> (e.from == n.id && e.to in wids) || (e.to == n.id && e.from in wids), st.edges)
        linked && push!(out, n.id)
    end
    out
end

function area_relevant_discoveries(st::State, z::Node)::Vector{String}
    surfaces = area_surface(st, z)
    tags = area_tags(st, z)
    cone = area_node_ids(st, z)
    scored = Tuple{Int,String}[]
    for discovery in listnodes(st, :y)
        discovery.status === :active || continue
        anchors = discovery_anchor_count(st, discovery, surfaces, tags, cone)
        anchors > 0 && push!(scored, (-anchors, discovery.id))
    end
    out = String[]
    for (_, id) in sort!(scored)
        push!(out, id)
    end
    out
end

function packet_cone(st::State, w::Node; depth::Int=4, maxcount::Int=50)::String
    back = backward_cone(st, w.id; depth=depth, maxcount=maxcount)
    fwd = forward_cone(st, w.id; depth=depth, maxcount=maxcount)
    io = IOBuffer()
    println(io)
    println(io, "## Contraction order")
    println(io)
    for (i, id) in enumerate(contraction_order(st, back.ids))
        n = get(st.nodes, id, nothing)
        n === nothing && continue
        println(io, i, ". ", id, "  ", n.status, "  ", n.title)
    end
    println(io)
    println(io, "## Forward cone (impact)")
    println(io)
    for id in fwd.ids
        n = get(st.nodes, id, nothing)
        n === nothing && continue
        println(io, "- ", id, "  ", n.status, "  ", n.title)
    end
    println(io)
    println(io, "## Fragility")
    println(io)
    for (g, k) in goal_fragility(st, w)
        if k == 0
            println(io, "- ", g, ": no blocks-path")
        elseif k == 1
            println(io, "- ", g, ": 1 (brittle)")
        else
            println(io, "- ", g, ": ", k, " disjoint blocks-paths")
        end
    end
    arts = relevant_discoveries(st, w, back.ids; maxcount=maxcount)
    if !isempty(arts)
        println(io)
        println(io, "## Relevant discoveries")
        println(io)
        for id in arts
            n = get(st.nodes, id, nothing)
            n === nothing && continue
            println(io, "- ", id, "  ", n.title)
        end
    end
    if back.truncated || fwd.truncated
        println(io)
        println(io, "> cone truncated (depth=", depth, ", max=", maxcount, ")")
    end
    String(take!(io))
end
