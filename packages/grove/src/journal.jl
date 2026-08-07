using JSON

const GROVE_JOURNAL_NAME = "journal.log"

journal_file(devdir_path::AbstractString)::String =
    joinpath(String(devdir_path), GROVE_JOURNAL_NAME)

"""After structural undo edits, rebuild `st.counters` from remaining ids / edges."""
function journal_reconcile_counters!(st::State)::Nothing
    empty!(st.counters)
    for nid in keys(st.nodes)
        record_id!(st, nid)
    end
    for e in st.edges
        record_id!(st, e.from)
        record_id!(st, e.to)
    end
    nothing
end

function append_journal_record!(journal_path::AbstractString, rec::AbstractDict)::Nothing
    mkpath(dirname(journal_path))
    open(journal_path, "a") do io
        println(io, JSON.json(rec))
    end
    nothing
end

"""Non-empty stripped lines and parsed records (paired 1:1)."""
function journal_read_nonempty_pairs(journal_path::AbstractString)::Tuple{Vector{String}, Vector{Dict{String,Any}}}
    rawlines = String[]
    recs = Dict{String,Any}[]
    !isfile(journal_path) && return (rawlines, recs)
    for line in eachline(journal_path)
        s = strip(line)
        isempty(s) && continue
        push!(rawlines, s)
        push!(recs, JSON.parse(s))
    end
    (rawlines, recs)
end

const JOURNAL_GATE_OP = "gate"
const JOURNAL_DISTILL_OP = "distill"
const JOURNAL_ARCHIVE_OP = "archive"
const JOURNAL_NONMUTATION_OPS =
    (JOURNAL_GATE_OP, JOURNAL_DISTILL_OP, JOURNAL_ARCHIVE_OP, "dor_reject", "undo")

function journal_record_mutation(rec)::Bool
    inv = get(rec, "inv", nothing)
    inv isa AbstractDict || return true
    !(String(get(inv, "op", "")) in JOURNAL_NONMUTATION_OPS)
end

function distill_null_attested(journal_path::AbstractString, gid::AbstractString)::Bool
    _, recs = journal_read_nonempty_pairs(journal_path)
    for rec in recs
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == JOURNAL_DISTILL_OP || continue
        String(get(inv, "goal", "")) == String(gid) || continue
        return true
    end
    false
end

function journal_tail_mutation_view(journal_path::AbstractString, n::Int)::Union{Nothing,Tuple{Vector{Int},Vector{Dict{String,Any}}}}
    (n <= 0) && return nothing
    _, recs = journal_read_nonempty_pairs(journal_path)
    idxs = Int[]
    for i in length(recs):-1:1
        journal_record_mutation(recs[i]) || continue
        push!(idxs, i)
        length(idxs) == n && break
    end
    length(idxs) < n && return nothing
    reverse!(idxs)
    (idxs, recs[idxs])
end

function journal_drop_lines_inplace!(journal_path::AbstractString, idxs::Vector{Int})::Nothing
    isempty(idxs) && return nothing
    rawlines, _ = journal_read_nonempty_pairs(journal_path)
    drop = Set(idxs)
    keep = String[rawlines[i] for i in eachindex(rawlines) if !(i in drop)]
    if isempty(keep)
        rm(journal_path; force=true)
    else
        write(journal_path, join(keep, "\n") * "\n")
    end
    nothing
end

function wrap_journal_record(cmd::AbstractString, inv::AbstractDict;
                             session::Union{Nothing,AbstractString}=nothing)::Dict{String,Any}
    rec = Dict{String,Any}("v" => 1, "ts" => utc_stamp_second(), "cmd" => String(cmd), "inv" => inv)
    session !== nothing && (rec["session"] = String(session))
    rec
end

journal_inverse_rm_node(id::AbstractString)::Dict{String,Any} =
    Dict{String,Any}("op" => "rm_node", "id" => String(id))

journal_inverse_of_link_forward(label::Symbol, from::AbstractString, to::AbstractString)::Dict =
    Dict("op" => "unlink_edge", "from" => String(from), "label" => String(label), "to" => String(to))

function journal_inverse_restore_edge(from::AbstractString, label::Symbol, to::AbstractString,
                                     tc)::Dict
    d = Dict{String,Any}(
        "op" => "restore_edge",
        "from" => String(from),
        "label" => String(label),
        "to" => String(to),
    )
    if tc isa AbstractString && !isempty(strip(tc))
        d["t_created"] = String(tc)
    else
        d["t_created"] = ""
    end
    d
end

journal_inverse_restore_fitness_key(wid::AbstractString, gid::AbstractString,
                                    had_key::Bool, previous::Union{Nothing,Int})::Dict =
    Dict{String,Any}(
        "op" => "restore_fitness_key",
        "wid" => String(wid),
        "gid" => String(gid),
        "had_key" => had_key,
        "previous" => previous === nothing ? nothing : Int(previous),
    )

"""Restore `session` / `session_at` attrs from a journal `inv` (no-op if keys absent)."""
function journal_restore_w_session_attrs_if_present!(w::Node, inv)::Nothing
    haskey(inv, "had_session_before") || return nothing
    if Bool(inv["had_session_before"])
        w.attrs["session"] = String(get(inv, "old_session", "")::Union{String,Any})
    else
        delete!(w.attrs, "session")
    end
    if haskey(inv, "had_session_at_before")
        if Bool(inv["had_session_at_before"])
            w.attrs["session_at"] = String(get(inv, "old_session_at", "")::Union{String,Any})
        else
            delete!(w.attrs, "session_at")
        end
    end
    nothing
end

function journal_apply_inverse!(st::State, inv)::Union{Nothing,String}
    op = get(inv, "op", "")::String

    function fail(msg::AbstractString)::String
        string("journal undo: ", msg)
    end

    if op == "rm_node"
        id = String(inv["id"])
        !haskey(st.nodes, id) && return nothing
        delete!(st.nodes, id)
        filter!(e -> !(e.from == id || e.to == id), st.edges)
        journal_reconcile_counters!(st)
        return nothing
    elseif op == "unlink_edge"
        from, lb, to = String(inv["from"]), Symbol(inv["label"]), String(inv["to"])
        n0 = length(st.edges)
        filter!(e -> !(e.from == from && e.label === lb && e.to == to), st.edges)
        length(st.edges) == n0 && return fail("unlink_edge: missing edge $(from) $(lb) $(to)")
        haskey(st.nodes, from) && stamp_touch_node!(st.nodes[from])
        haskey(st.nodes, to) && stamp_touch_node!(st.nodes[to])
        return nothing
    elseif op == "restore_edge"
        from, lb, to = String(inv["from"]), Symbol(inv["label"]), String(inv["to"])
        if any(e -> e.from == from && e.label === lb && e.to == to, st.edges)
            return nothing
        end
        r = validate_and_push_edge!(st, from, lb, to)
        r !== nothing && return fail(r)
        ee = nothing
        for ed in reverse(st.edges)
            if ed.from == from && ed.label === lb && ed.to == to
                ee = ed
                break
            end
        end
        ee === nothing && return fail("restore_edge: edge missing after validate")
        tc = strip(String(get(inv, "t_created", "")))
        if isempty(tc)
            ee.t_created = nothing
        else
            ee.t_created = tc
        end
        return nothing
    elseif op == "set_cynefin"
        n = st.nodes[String(inv["id"])]
        o = inv["old"]
        n.cynefin = (!haskey(inv, "old") || o === nothing || isempty(String(o))) ? nothing : Symbol(String(o))
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_type"
        n = st.nodes[String(inv["id"])]
        o = inv["old"]
        n.type = (!haskey(inv, "old") || o === nothing || isempty(String(o))) ? nothing : Symbol(String(o))
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_title"
        n = st.nodes[String(inv["id"])]
        n.title = String(get(inv, "old", "")::Union{String,Any})
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_g_attr_fitness"
        n = st.nodes[String(inv["id"])]
        n.attrs["fitness"] = String(get(inv, "old", "")::Union{String,Any})
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_g_attr_fitness_kind"
        n = st.nodes[String(inv["id"])]
        if Bool(inv["had_before"])
            n.attrs["fitness_kind"] = String(inv["old"])
        else
            delete!(n.attrs, "fitness_kind")
        end
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_requires_coverage"
        n = st.nodes[String(inv["id"])]
        if Bool(inv["had_before"])
            n.attrs["requires_coverage"] = String(inv["old"])
        else
            delete!(n.attrs, "requires_coverage")
        end
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_g_area"
        n = st.nodes[String(inv["id"])]
        if Bool(inv["had_before"])
            n.fields[:area] = String(inv["old"])
        else
            delete!(n.fields, :area)
        end
        stamp_touch_node!(n)
        return nothing
    elseif op == "set_status_plain"
        n = st.nodes[String(inv["id"])]
        n.status = Symbol(String(inv["old_status"]))
        stamp_touch_node!(n)
        n.kind === :w && rederive_goals!(st, n)
        return nothing
    elseif op == "set_w_status_with_goals"
        gs = inv["goal_statuses"]
        gs isa AbstractDict || return fail("missing goal_statuses")
        for (gidv, sv) in gs
            gid = String(gidv)
            haskey(st.nodes, gid) || return fail("goal node missing $(gid)")
            st.nodes[gid].status = Symbol(String(sv))
            stamp_touch_node!(st.nodes[gid])
        end
        w = st.nodes[String(inv["id"])]
        w.status = Symbol(String(inv["old_w_status"]))
        journal_restore_w_session_attrs_if_present!(w, inv)
        stamp_touch_node!(w)
        rederive_goals!(st, w)
        return nothing
    elseif op == "session_restore_claim"
        w = st.nodes[String(inv["id"])]
        journal_restore_w_session_attrs_if_present!(w, inv)
        stamp_touch_node!(w)
        return nothing
    elseif op == "field_pop_last"
        n = st.nodes[String(inv["id"])]
        fsym = Symbol(String(inv["field"]))
        v = get_vector_field!(n, fsym)
        isempty(v) && return fail("field_pop_last empty $(fsym)")
        pop!(v)
        stamp_touch_node!(n)
        return nothing
    elseif op == "field_restore_lines"
        n = st.nodes[String(inv["id"])]
        fsym = Symbol(String(inv["field"]))
        arr = map(String, collect(inv["lines"]))
        form = FIELD_CATALOG[(n.kind, fsym)]
        (form === :prose || form === :reflist) || return fail("field_restore_lines wrong form")
        n.fields[fsym] = arr
        stamp_touch_node!(n)
        return nothing
    elseif op == "field_restore_fitness"
        n = st.nodes[String(inv["id"])]
        fsym = Symbol(String(inv["field"]))
        d = Dict{String,Int}()
        for (k0, vv) in inv["map"]
            d[String(k0)] = Int(round(vv))
        end
        n.fields[fsym] = d
        stamp_touch_node!(n)
        return nothing
    elseif op == "field_restore_single"
        n = st.nodes[String(inv["id"])]
        fsym = Symbol(String(inv["field"]))
        n.fields[fsym] = String(inv["value"])
        stamp_touch_node!(n)
        return nothing
    elseif op == "field_insert_line"
        n = st.nodes[String(inv["id"])]
        fsym = Symbol(String(inv["field"]))
        idx = Int(round(inv["index"]))
        v = get_vector_field!(n, fsym)
        (idx < 1 || idx > length(v) + 1) && return fail("field_insert_line bad index $(idx)")
        insert!(v, idx, String(inv["line"]))
        stamp_touch_node!(n)
        return nothing
    elseif op == "restore_fitness_key"
        w = st.nodes[String(inv["wid"])]
        gid = String(inv["gid"])
        fid = get!(w.fields, :fitness, Dict{String,Int}())
        if Bool(inv["had_key"])
            prev = inv["previous"]
            prev === nothing && return fail("restore_fitness_key missing previous")
            fid[gid] = Int(round(prev))
        else
            delete!(fid, gid)
        end
        stamp_touch_node!(w)
        return nothing
    elseif op == "renumber_swap"
        apply_renumber!(st, String(inv["from"]), String(inv["to"]))
        return nothing
    elseif op == "revalidate_restore"
        n = st.nodes[String(inv["id"])]
        n.status = Symbol(String(inv["old_status"]))
        if Bool(get(inv, "had_surface", false))
            n.fields[:surface] = String[String(s) for s in get(inv, "old_surface", Any[])]
        else
            delete!(n.fields, :surface)
        end
        rv = get(n.fields, :revalidation, nothing)
        rv isa Vector && !isempty(rv) && pop!(rv)
        for ed in get(inv, "added_edges", Any[])
            f0, l0, t0 = String(ed["from"]), Symbol(String(ed["label"])), String(ed["to"])
            filter!(e -> !(e.from == f0 && e.label === l0 && e.to == t0), st.edges)
        end
        stamp_touch_node!(n)
        return nothing
    elseif op == "glossary_rename_restore"
        tg = get(inv, "tags", nothing)
        tg isa AbstractDict || return fail("glossary_rename_restore missing tags")
        for (id0, lines0) in tg
            n = get(st.nodes, String(id0), nothing)
            n === nothing && continue
            n.fields[:tags] = String[String(x0) for x0 in lines0]
            stamp_touch_node!(n)
        end
        return nothing
    else
        return fail("unknown inverse op `$op`")
    end
end

function get_vector_field!(n::Node, fname::Symbol)::Vector{String}
    v = get(n.fields, fname, nothing)
    v isa Vector || (get!(n.fields, fname, String[]); v = n.fields[fname]::Vector)
    vv = String[e isa String ? e : string(e) for e in v]
    n.fields[fname] = vv
    vv
end
