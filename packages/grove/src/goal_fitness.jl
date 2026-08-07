"""Structured fitness on goals (kind + target/current) plus legacy `fitness` attr."""

const GOAL_FITNESS_KINDS =
    (:count, :ratio, :boolean, :metric, :manual)

"""Active only when attr `fitness_kind` parses to one of [`GOAL_FITNESS_KINDS`]; else legacy."""
function goal_structured_kind(g::Node)::Union{Nothing,Symbol}
    g.kind !== :g && return nothing
    s = strip(get(g.attrs, "fitness_kind", ""))
    isempty(s) && return nothing
    sy = Symbol(s)
    sy in GOAL_FITNESS_KINDS ? sy : nothing
end

function parse_fitness_target(label::AbstractString)::Union{Int,Nothing}
    m = match(r"(\d+)\s*/\s*(\d+)", label)
    m === nothing ? nothing : parse(Int, m.captures[2])
end

"""Sum of staged `done` fitness deltas referencing `gid`."""
function aggregate_fitness_delta(st::State, gid::AbstractString)::Int
    g0 = String(gid)
    t = 0
    for ww in listnodes(st, :w)
        ww.status === :done || continue
        fd = get(ww.fields, :fitness, Dict{String,Int}())
        haskey(fd, g0) && (t += fd[g0])
    end
    t
end

function _parse_nonneg_int(val::AbstractString)::Union{Nothing,Int}
    v = tryparse(Int, strip(String(val)))
    (v === nothing || v < 0) ? nothing : Int(v)
end

function _sync_goal_fitness_current_field!(g::Node, kind::Symbol, total::Int)::Nothing
    if kind === :boolean
        g.fields[:fitness_current] = total >= 1 ? "true" : "false"
    else
        g.fields[:fitness_current] = string(total)
    end
    nothing
end

"""Legacy: `attrs["fitness"]` denominator style + sum of deltas; no structured fields touched."""
function _refresh_goal_legacy!(g::Node, total::Int)::Nothing
    prev = g.status
    label = get(g.attrs, "fitness", "")
    target = parse_fitness_target(label)
    if target !== nothing && total >= target
        g.status = :verified
    elseif total > 0
        g.status = :partial
    end
    g.status !== prev && stamp_touch_node!(g)
    nothing
end

"""Structured kinds: writes `fitness_current`, updates goal `status` except `manual`."""
function refresh_goal_structured_fitness!(st::State, g::Node)::Nothing
    g.kind !== :g && return nothing
    kind = goal_structured_kind(g)
    total = aggregate_fitness_delta(st, g.id)
    if kind === nothing
        _refresh_goal_legacy!(g, total)
        return nothing
    end

    if kind === :manual
        return nothing
    end

    _sync_goal_fitness_current_field!(g, kind, total)

    prev = g.status
    tgt_txt = strip(string(get(g.fields, :fitness_target, "")))

    if kind === :boolean
        if total >= 1
            g.status = :verified
        end
    elseif kind === :count
        ntar = isempty(tgt_txt) ? nothing : _parse_nonneg_int(tgt_txt)
        if ntar !== nothing
            if total >= ntar
                g.status = :verified
            elseif total > 0
                g.status = :partial
            end
        end
    elseif kind === :ratio
        ntar = parse_fitness_target(tgt_txt)
        ntar === nothing && !isempty(tgt_txt) && (ntar = _parse_nonneg_int(tgt_txt))
        if ntar !== nothing
            if total >= ntar
                g.status = :verified
            elseif total > 0
                g.status = :partial
            end
        end
    elseif kind === :metric
        ntar = isempty(tgt_txt) ? nothing : _parse_nonneg_int(tgt_txt)
        if ntar !== nothing
            if total >= ntar
                g.status = :verified
            elseif total > 0
                g.status = :partial
            end
        end
    end
    g.status !== prev && stamp_touch_node!(g)
    nothing
end

function rederive_goals!(st::State, w::Node)::Nothing
    w.status === :done || return nothing
    seen = Set{String}()
    for gid0 in get(w.fields, :goals, String[])
        gid = String(strip(string(gid0)))
        isempty(gid) && continue
        gid in seen && continue
        push!(seen, gid)
        gg = get(st.nodes, gid, nothing)
        gg === nothing && continue
        gg.kind === :g || continue
        refresh_goal_structured_fitness!(st, gg)
    end
    nothing
end

function goal_fitness_table_cell(g::Node)::String
    k = strip(get(g.attrs, "fitness_kind", ""))
    if isempty(k)
        return strip(string(get(g.attrs, "fitness", "")))
    end
    cur = strip(string(get(g.fields, :fitness_current, "")))
    tgt = strip(string(get(g.fields, :fitness_target, "")))
    parts = String[k]
    if !isempty(cur) || !isempty(tgt)
        push!(parts, isempty(tgt) ? string("current=", cur) : string("current=", cur, " target=", tgt))
    elseif !isempty(strip(string(get(g.attrs, "fitness", ""))))
        push!(parts, strip(string(get(g.attrs, "fitness", ""))))
    end
    join(parts, "; ")
end
