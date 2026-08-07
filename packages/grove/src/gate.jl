const GateBaseline = NamedTuple{(:ts, :tw, :dones),Tuple{String,Int,Int}}

function gate_baseline(recs::Vector{Dict{String,Any}})::Union{Nothing,GateBaseline}
    for rec in Iterators.reverse(recs)
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == JOURNAL_GATE_OP || continue
        ts = String(strip(String(get(rec, "ts", ""))))
        isempty(ts) && continue
        return (ts=ts, tw=Int(get(inv, "tw", 0)), dones=Int(get(inv, "dones", 0)))
    end
    nothing
end

gate_time_cut(baseline)::String = baseline === nothing ? "" : baseline.ts

function gate_done_since(st::State, cut::AbstractString)::Vector{Node}
    out = Node[]
    for w in listnodes(st, :w)
        w.status === :done || continue
        get(w.attrs, "t_updated", "") >= cut || continue
        push!(out, w)
    end
    out
end

gate_dones(st::State, baseline)::Int = length(gate_done_since(st, gate_time_cut(baseline)))

function gate_git_root_ok(root::AbstractString)::Bool
    Sys.which("git") === nothing && return false
    git_repository_root(root)
end

function gate_git_files_by_w(root::AbstractString, wids::Vector{String}, cut::AbstractString)::Dict{String,Vector{String}}
    out = Dict{String,Vector{String}}(w => String[] for w in wids)
    (isempty(wids) || !gate_git_root_ok(root)) && return out
    args = String["git", "-C", abspath(String(root)), "--no-pager", "log", "--name-only", "--pretty=format:\x01%s"]
    isempty(cut) || push!(args, "--since=$(String(cut))")
    txt = withenv("GIT_TERMINAL_PROMPT" => "0") do
        try
            read(Cmd(args), String)
        catch
            ""
        end
    end
    want = Set{String}(wids)
    idre = r"\b([A-Z]-[0-9]+)\b"
    hits = String[]
    for line in eachline(IOBuffer(txt))
        if startswith(line, "\x01")
            hits = String[]
            for m in eachmatch(idre, line)
                id = String(m.captures[1])
                (id in want && !(id in hits)) && push!(hits, id)
            end
            continue
        end
        s = strip(line)
        isempty(s) && continue
        for id in hits
            push!(out[id], s)
        end
    end
    for id in keys(out)
        out[id] = sort!(unique(out[id]))
    end
    out
end

function surface_overflows(st::State, root::AbstractString, baseline;
                           theta::Int=0)::Vector{Tuple{String,Vector{String}}}
    out = Tuple{String,Vector{String}}[]
    cut = gate_time_cut(baseline)
    dones = gate_done_since(st, cut)
    by_w = gate_git_files_by_w(root, String[w.id for w in dones], cut)
    for w in dones
        actual = by_w[w.id]
        isempty(actual) && continue
        declared = Set{String}(String(x) for x in get(w.fields, :surface, String[]))
        overflow = sort!(String[x for x in actual if !(x in declared)])
        length(overflow) > theta && push!(out, (w.id, overflow))
    end
    out
end

function gate_invalidated(st::State, cut::AbstractString)::Vector{Node}
    out = Node[]
    for b in listnodes(st, :b)
        b.status in (:invalidated_acceptable, :invalidated_blocking) || continue
        get(b.attrs, "t_updated", "") >= cut || continue
        push!(out, b)
    end
    out
end

function gate_accepted(st::State, cut::AbstractString)::Vector{Node}
    out = Node[]
    for d in listnodes(st, :d)
        d.status === :accepted || continue
        get(d.attrs, "t_updated", "") >= cut || continue
        push!(out, d)
    end
    out
end

function gate_report(st::State, recs::Vector{Dict{String,Any}}, root::AbstractString;
                     theta::Int=0, n::Int=5)
    baseline = gate_baseline(recs)
    cut = gate_time_cut(baseline)
    tw_now = treewidth_upper(st)
    tw_delta = baseline === nothing ? 0 : tw_now - baseline.tw
    dones = length(gate_done_since(st, cut))
    overflows = surface_overflows(st, root, baseline; theta=theta)
    invalidated = gate_invalidated(st, cut)
    accepted = gate_accepted(st, cut)
    empty = tw_delta == 0 && isempty(overflows) && isempty(invalidated) && isempty(accepted)
    (baseline=baseline, tw_now=tw_now, tw_delta=tw_delta, dones=dones, due=dones >= n,
     overflows=overflows, invalidated=invalidated, accepted=accepted, empty=empty,
     theta=theta, n=n)
end

function gate_json_payload(rep)::Dict{String,Any}
    Dict{String,Any}(
        "command" => "gate",
        "baseline" => rep.baseline === nothing ? nothing : Dict{String,Any}(
            "ts" => rep.baseline.ts, "tw" => rep.baseline.tw, "dones" => rep.baseline.dones),
        "tw_now" => rep.tw_now,
        "tw_delta" => rep.tw_delta,
        "dones" => rep.dones,
        "due" => rep.due,
        "overflows" => [Dict{String,Any}("w" => wid, "paths" => paths) for (wid, paths) in rep.overflows],
        "invalidated" => [Dict{String,Any}("id" => b.id, "title" => b.title, "status" => string(b.status)) for b in rep.invalidated],
        "accepted" => [Dict{String,Any}("id" => d.id, "title" => d.title) for d in rep.accepted],
        "empty" => rep.empty,
        "theta" => rep.theta,
        "n" => rep.n,
    )
end
