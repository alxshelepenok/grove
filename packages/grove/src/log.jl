using JSON

struct LogTimelineRow
    ts::String
    tiebreaker::String
    line::String
end

function _log_ts(n::Node, key::AbstractString)::String
    strip(String(get(n.attrs, key, "")))
end

function _blank_ts(s::AbstractString)::Bool
    isempty(strip(s))
end

function _journal_collect_value_strings!(x, acc::Vector{String})::Nothing
    if x isa AbstractDict
        for v in values(x)
            _journal_collect_value_strings!(v, acc)
        end
    elseif x isa AbstractVector
        for v in x
            _journal_collect_value_strings!(v, acc)
        end
    elseif x isa Union{AbstractString,Symbol}
        push!(acc, strip(string(x)))
    elseif x isa Union{Number,Bool}
        push!(acc, strip(string(x)))
    elseif x === nothing
    end
    nothing
end

function journal_inv_mentions_id(inv::AbstractDict, needle::AbstractString)::Bool
    acc = String[]
    _journal_collect_value_strings!(inv, acc)
    n = String(strip(needle))
    any(==(n), acc)
end

function journal_file_mentions_id(journal_path::AbstractString, needle::AbstractString)::Bool
    isfile(journal_path) || return false
    text = try
        read(journal_path, String)
    catch
        return false
    end
    n = String(strip(needle))
    for line in eachline(IOBuffer(text))
        s = strip(line)
        isempty(s) && continue
        rec = try
            JSON.parse(s)
        catch
            continue
        end
        rec isa AbstractDict || continue
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        journal_inv_mentions_id(inv, n) && return true
    end
    false
end

function append_journal_timeline!(rows::Vector{LogTimelineRow}, journal_path::AbstractString,
    filt::Union{Nothing,String})::Nothing
    isfile(journal_path) || return nothing
    text = try
        read(journal_path, String)
    catch
        return nothing
    end
    li = 0
    for line in eachline(IOBuffer(text))
        s = strip(line)
        isempty(s) && continue
        rec = try
            JSON.parse(s)
        catch
            continue
        end
        rec isa AbstractDict || continue
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        filt !== nothing && !journal_inv_mentions_id(inv, filt) && continue
        li += 1
        cmd = String(get(rec, "cmd", "?"))
        ts0 = get(rec, "ts", nothing)
        ts = ts0 isa AbstractString && !isempty(strip(ts0)) ?
             String(strip(ts0)) : "1980-01-01T00:00:00Z"
        tb = "journal " * lpad(string(li), 9, '0')
        invop = String(get(inv, "op", ""))
        brief_parts = String[invop]
        for k in ("id", "wid", "from", "to", "gid", "goal")
            haskey(inv, k) || continue
            v = inv[k]
            v === nothing && continue
            push!(brief_parts, "$(k)=$(v)")
        end
        brief = join(brief_parts, ' ')
        ln = "$ts\tjournal\t$cmd\t$brief"
        push!(rows, LogTimelineRow(ts, tb, ln))
    end
    nothing
end

"""
log_timeline(st; idfilt=nothing, limit=200, journal_path=nothing)

Newest first. Merges optional `journal_path` JSON-lines with node/edge `t_*` rows.
When `idfilt` is set, keep only matching node, edge, or journal rows.
`limit <= 0` means no cap.
"""
function log_timeline(st::State; idfilt::Union{Nothing,String}=nothing, limit::Int=200,
    journal_path::Union{Nothing,String}=nothing)::Vector{LogTimelineRow}
    rows = LogTimelineRow[]

    filt = idfilt !== nothing ? String(strip(idfilt)) : nothing
    emit_node(n::Node) =
        begin
            filt !== nothing && n.id != filt && return
            tc = _log_ts(n, "t_created")
            tu = _log_ts(n, "t_updated")
            if _blank_ts(tc) && _blank_ts(tu)
                return
            elseif _blank_ts(tc)
                tc = tu
            elseif _blank_ts(tu)
                tu = tc
            end
            ttl = isempty(n.title) ? "(no title)" : n.title
            push!(rows, LogTimelineRow(tc, "$(String(n.kind)) $(n.id) tc",
                "$tc\tnode\t$(String(n.kind))\t$(n.id)\tcreated\t$ttl status=$(String(n.status))"))
            if tu != tc
                push!(rows, LogTimelineRow(tu, "$(String(n.kind)) $(n.id) tu",
                    "$tu\tnode\t$(String(n.kind))\t$(n.id)\tupdated\t$ttl status=$(String(n.status))"))
            end
            nothing
        end

    emit_edge(e::Edge) =
        begin
            filt !== nothing && filt != e.from && filt != e.to && return
            ts = e.t_created === nothing ? "" : strip(String(e.t_created))
            _blank_ts(ts) && return nothing
            push!(rows, LogTimelineRow(ts, "edge $(e.from) $(String(e.label)) $(e.to)",
                "$ts\tedge\t$(e.from)\t$(String(e.label))\t$(e.to)"))
            nothing
        end

    for n in values(st.nodes)
        emit_node(n)
    end
    for e in st.edges
        emit_edge(e)
    end

    if journal_path !== nothing
        append_journal_timeline!(rows, String(journal_path), filt)
    end

    sort!(rows; by=r -> (r.ts, r.tiebreaker), rev=true)
    limit > 0 && length(rows) > limit ? rows[1:limit] : rows
end

journalpath(ctx) = journal_file(devdir(ctx))

function print_timeline(rows::AbstractVector{LogTimelineRow}; io::IO=stdout)::Nothing
    for r in rows
        println(io, r.line)
    end
    nothing
end
