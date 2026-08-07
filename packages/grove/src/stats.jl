const STATS_INTERVAL =
    NamedTuple{(:start, :stop, :status),Tuple{Union{Nothing,Dates.DateTime},Dates.DateTime,Symbol}}

const STATS_STATUS_OPS = ("set_status_plain", "set_w_status_with_goals", "revalidate_restore")

function stats_parse_ts(s)::Union{Nothing,Dates.DateTime}
    t = strip(String(s))
    endswith(t, "Z") || return nothing
    try
        return Dates.DateTime(t[1:end-1])
    catch
        return nothing
    end
end

function stats_median(xs::Vector{Float64})::Float64
    ys = sort(xs)
    n = length(ys)
    isodd(n) ? ys[(n + 1) ÷ 2] : (ys[n ÷ 2] + ys[n ÷ 2 + 1]) / 2
end

function stats_intervals(st::State, recs, now_dt::Dates.DateTime)
    tracked = Dict{String,Symbol}(id => n.status for (id, n) in st.nodes)
    cursor = Dict{String,Dates.DateTime}(id => now_dt for (id, _) in st.nodes)
    birth = Dict{String,Dates.DateTime}()
    touched = Set{String}()
    ivals = Dict{String,Vector{STATS_INTERVAL}}(id => STATS_INTERVAL[] for (id, _) in st.nodes)
    oldest = nothing
    for rec in recs
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing && continue
        (oldest === nothing || ts < oldest) && (oldest = ts)
    end
    for rec in Iterators.reverse(recs)
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing && continue
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        op = String(get(inv, "op", ""))
        id = String(get(inv, "id", ""))
        if op in STATS_STATUS_OPS && haskey(tracked, id)
            old = op == "set_w_status_with_goals" ? String(get(inv, "old_w_status", "")) :
                  String(get(inv, "old_status", ""))
            push!(ivals[id], (start=ts, stop=cursor[id], status=tracked[id]))
            tracked[id] = Symbol(old)
            cursor[id] = ts
            push!(touched, id)
        elseif op == "rm_node" && String(get(rec, "cmd", "")) == "add" && haskey(st.nodes, id)
            birth[id] = ts
            push!(touched, id)
        end
    end
    for (id, _) in st.nodes
        start = if haskey(birth, id)
            birth[id]
        elseif id in touched
            oldest
        else
            nothing
        end
        push!(ivals[id], (start=start, stop=cursor[id], status=tracked[id]))
        reverse!(ivals[id])
    end
    ivals
end

function stats_cycle_time(st::State, ivals)
    by_class = Dict{String,Vector{Int}}()
    all_seconds = Int[]
    for n in listnodes(st, :w; include_archived=true)
        ivs = ivals[n.id]
        tr = [i.start for i in ivs if i.status === :ready && i.start !== nothing]
        td = [i.start for i in ivs if i.status === :done && i.start !== nothing]
        (isempty(tr) || isempty(td)) && continue
        t0 = minimum(tr)
        t1 = minimum(td)
        t1 < t0 && continue
        secs = Dates.value(Dates.Second(t1 - t0))
        cls = n.cynefin === nothing ? "none" : string(n.cynefin)
        push!(get!(by_class, cls, Int[]), secs)
        push!(all_seconds, secs)
    end
    classes = Dict{String,Any}()
    for (cls, secs) in by_class
        hrs = Float64[s / 3600 for s in secs]
        classes[cls] = Dict{String,Any}(
            "n" => length(secs),
            "mean_hours" => sum(hrs) / length(hrs),
            "median_hours" => stats_median(hrs),
            "max_hours" => maximum(hrs),
        )
    end
    (classes, all_seconds)
end

function stats_dor(st::State, recs, ivals)
    total = 0
    per_node = Dict{String,Int}()
    reject_ts = Dict{String,Vector{Dates.DateTime}}()
    for rec in recs
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "dor_reject" || continue
        id = String(get(inv, "id", ""))
        total += 1
        per_node[id] = get(per_node, id, 0) + 1
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing || push!(get!(reject_ts, id, Dates.DateTime[]), ts)
    end
    entries = 0
    first_pass = 0
    for n in listnodes(st, :w; include_archived=true)
        for i in ivals[n.id]
            (i.status === :progress && i.start !== nothing) || continue
            entries += 1
            rts = get(reject_ts, n.id, Dates.DateTime[])
            any(t -> t < i.start, rts) || (first_pass += 1)
        end
    end
    (total, per_node, entries, first_pass, entries == 0 ? nothing : first_pass / entries)
end

function stats_bets(st::State, ivals)
    counts = Dict{Symbol,Int}(
        :validated => 0, :invalidated_acceptable => 0, :invalidated_blocking => 0)
    for n in listnodes(st, :b; include_archived=true)
        for i in ivals[n.id]
            i.start === nothing && continue
            haskey(counts, i.status) && (counts[i.status] += 1)
        end
    end
    den = counts[:invalidated_acceptable] + counts[:invalidated_blocking]
    (counts, den == 0 ? nothing : counts[:validated] / den)
end

function stats_discovery(st::State, recs, ivals)
    stale_entries = 0
    for n in listnodes(st, :y; include_archived=true)
        for i in ivals[n.id]
            (i.status === :stale && i.start !== nothing) && (stale_entries += 1)
        end
    end
    revalidations = 0
    gate_runs = 0
    gate_empty = 0
    overflow_events = 0
    invalidated_events = 0
    gates = Dict{String,Any}[]
    for rec in recs
        String(get(rec, "cmd", "")) == "revalidate" && (revalidations += 1)
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "gate" || continue
        gate_runs += 1
        get(inv, "empty", false) === true && (gate_empty += 1)
        ov = get(inv, "overflows", nothing)
        ov isa AbstractVector && (overflow_events += length(ov))
        ivl = get(inv, "invalidated", nothing)
        ivl isa AbstractVector && (invalidated_events += length(ivl))
        oc = get(inv, "overflow_counts", nothing)
        push!(gates, Dict{String,Any}(
            "ts" => String(get(rec, "ts", "")),
            "tw" => get(inv, "tw", 0),
            "dones" => get(inv, "dones", 0),
            "empty" => get(inv, "empty", false),
            "overflow_events" => ov isa AbstractVector ? length(ov) : 0,
            "overflow_paths" => oc isa AbstractDict ? sum(Int(v) for v in values(oc); init=0) : nothing,
            "invalidated_events" => ivl isa AbstractVector ? length(ivl) : 0,
        ))
    end
    (stale_entries, revalidations, gate_runs, gate_empty, overflow_events, invalidated_events, gates)
end

function stats_undo(recs)
    events = 0
    steps = 0
    mutations = 0
    for rec in recs
        journal_record_mutation(rec) && (mutations += 1)
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "undo" || continue
        events += 1
        steps += Int(get(inv, "steps", 0))
    end
    (events, steps, mutations, mutations == 0 ? nothing : 100 * events / mutations)
end

function stats_sessions(recs)
    per = Dict{String,Int}()
    for rec in recs
        tok = strip(String(get(rec, "session", "unknown")))
        isempty(tok) && (tok = "unknown")
        per[tok] = get(per, tok, 0) + 1
    end
    entries = Dict{String,Any}[
        Dict{String,Any}("session" => t, "commands" => per[t]) for t in sort!(collect(keys(per)))]
    counts = collect(values(per))
    summary = isempty(counts) ? (nothing, nothing, nothing) :
              (sum(counts) / length(counts), stats_median(Float64.(counts)), maximum(counts))
    (entries, summary...)
end

function stats_hours_summary(hrs::Vector{Float64})::Dict{String,Any}
    Dict{String,Any}(
        "n" => length(hrs),
        "mean_hours" => isempty(hrs) ? nothing : sum(hrs) / length(hrs),
        "median_hours" => isempty(hrs) ? nothing : stats_median(hrs),
        "max_hours" => isempty(hrs) ? nothing : maximum(hrs),
    )
end

function stats_checkpoint_latency(st::State, recs, ivals)
    dor_hrs = Float64[]
    for rec in recs
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "dor_reject" || continue
        id = String(get(inv, "id", ""))
        haskey(ivals, id) || continue
        rts = stats_parse_ts(get(rec, "ts", ""))
        rts === nothing && continue
        starts = Dates.DateTime[i.start for i in ivals[id]
                                if i.status === :progress && i.start !== nothing && i.start > rts]
        isempty(starts) && continue
        push!(dor_hrs, Dates.value(Dates.Millisecond(minimum(starts) - rts)) / 3600000)
    end
    disc_hrs = Float64[]
    for n in listnodes(st, :y; include_archived=true)
        t0s = Dates.DateTime[i.start for i in ivals[n.id]
                             if i.status === :proposed && i.start !== nothing]
        t1s = Dates.DateTime[i.start for i in ivals[n.id]
                             if i.status === :active && i.start !== nothing]
        (isempty(t0s) || isempty(t1s)) && continue
        t0, t1 = minimum(t0s), minimum(t1s)
        t1 < t0 && continue
        push!(disc_hrs, Dates.value(Dates.Millisecond(t1 - t0)) / 3600000)
    end
    (dor_hrs, disc_hrs)
end

function stats_post_approval_invalidation(st::State, ivals)
    ever_validated = 0
    invalidated = 0
    for n in listnodes(st, :b; include_archived=true)
        ivs = ivals[n.id]
        k = findfirst(i -> i.status === :validated, ivs)
        k === nothing && continue
        ever_validated += 1
        any(i -> i.status in (:invalidated_acceptable, :invalidated_blocking), ivs[k+1:end]) &&
            (invalidated += 1)
    end
    (invalidated, ever_validated, ever_validated == 0 ? nothing : invalidated / ever_validated)
end

function stats_rework(st::State, reject_per_node)::Dict{String,Any}
    covered_surfaces = Set{String}()
    for y in listnodes(st, :y)
        y.status === :active || continue
        for s in get(y.fields, :surface, String[])
            push!(covered_surfaces, String(s))
        end
    end
    out = Dict{String,Any}()
    for (key, want_covered) in (("covered", true), ("uncovered", false))
        rows = Tuple{String,Int}[]
        for w in listnodes(st, :w; include_archived=true)
            surf = String[String(s) for s in get(w.fields, :surface, String[])]
            covered = !isempty(surf) && any(s -> s in covered_surfaces, surf)
            covered == want_covered || continue
            push!(rows, (w.id, get(reject_per_node, w.id, 0)))
        end
        total = sum(r for (_, r) in rows; init=0)
        out[key] = Dict{String,Any}(
            "w" => length(rows),
            "rejects" => total,
            "mean_rejects" => isempty(rows) ? nothing : round(total / length(rows); digits=2),
            "per_w" => [Dict{String,Any}("id" => id, "rejects" => r) for (id, r) in rows],
        )
    end
    out
end

function stats_distill_yield(st::State, recs)
    goals = Node[g for g in listnodes(st, :g; include_archived=true) if g.archived]
    null_attested = Set{String}()
    for rec in recs
        String(get(rec, "cmd", "")) == "distill" || continue
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        gid = String(get(inv, "goal", ""))
        isempty(gid) || push!(null_attested, gid)
    end
    entries = Dict{String,Any}[]
    if !isempty(goals)
        st_open = deepcopy(st)
        for n in values(st_open.nodes)
            n.archived = false
        end
        for g in goals
            pool = exclusive_archive_ids(st_open, g.id)
            yids = Set{String}()
            for e in st.edges
                e.label === :distills || continue
                e.to in pool || continue
                yn = get(st.nodes, e.from, nothing)
                yn === nothing && continue
                yn.kind === :y || continue
                push!(yids, e.from)
            end
            ys = sort!(collect(yids))
            status = !isempty(ys) ? "real" : (g.id in null_attested ? "null" : "none")
            push!(entries, Dict{String,Any}(
                "goal" => g.id,
                "status" => status,
                "discoveries" => ys,
            ))
        end
    end
    (count(e -> e["status"] == "real", entries),
     count(e -> e["status"] == "null", entries),
     count(e -> e["status"] == "none", entries),
     entries)
end

function stats_dor_split(st::State, recs, ivals)
    reject_ts = Dict{String,Vector{Dates.DateTime}}()
    for rec in recs
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "dor_reject" || continue
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing && continue
        push!(get!(reject_ts, String(get(inv, "id", "")), Dates.DateTime[]), ts)
    end
    mutations = Tuple{Dates.DateTime,Dict{String,Any}}[]
    for rec in recs
        journal_record_mutation(rec) || continue
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing && continue
        push!(mutations, (ts, rec))
    end
    counts = Dict{String,Int}("no_reject" => 0, "reject_discovery" => 0, "reject_plain" => 0)
    for n in listnodes(st, :w; include_archived=true)
        for i in ivals[n.id]
            (i.status === :progress && i.start !== nothing) || continue
            prior = filter(t -> t < i.start, get(reject_ts, n.id, Dates.DateTime[]))
            if isempty(prior)
                counts["no_reject"] += 1
                continue
            end
            latest = maximum(prior)
            discovery = any(mutations) do (ts, rec)
                (latest < ts < i.start) || return false
                inv = get(rec, "inv", nothing)
                inv isa AbstractDict || return false
                id = String(get(inv, "id", ""))
                isempty(id) && return false
                nd = get(st.nodes, id, nothing)
                nd !== nothing && nd.kind in (:q, :b, :y)
            end
            counts[discovery ? "reject_discovery" : "reject_plain"] += 1
        end
    end
    den = counts["reject_discovery"] + counts["reject_plain"]
    (counts, den == 0 ? nothing : counts["reject_discovery"] / den)
end

function stats_cv_series(st::State, recs, now_ts::AbstractString)
    r = deepcopy(st)
    h0 = content_health(r)
    series = Dict{String,Any}[
        Dict{String,Any}("ts" => String(now_ts), "c" => sum(values(h0.c)), "v" => sum(values(h0.v)))]
    failures = 0
    for rec in Iterators.reverse(recs)
        journal_record_mutation(rec) || continue
        h = try
            msg = journal_apply_inverse!(r, get(rec, "inv", nothing))
            msg === nothing ? content_health(r) : nothing
        catch
            nothing
        end
        if h === nothing
            failures += 1
            continue
        end
        push!(series, Dict{String,Any}(
            "ts" => String(get(rec, "ts", "")), "c" => sum(values(h.c)), "v" => sum(values(h.v))))
    end
    reverse!(series)
    (series, failures)
end

function stats_surprise_series(st::State, recs, ivals, series)::Vector{Dict{String,Any}}
    events = Dates.DateTime[]
    for n in listnodes(st, :b; include_archived=true)
        for i in ivals[n.id]
            i.status in (:invalidated_acceptable, :invalidated_blocking) || continue
            i.start === nothing && continue
            push!(events, i.start)
        end
    end
    for rec in recs
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || continue
        String(get(inv, "op", "")) == "gate" || continue
        ov = get(inv, "overflows", nothing)
        ov isa AbstractVector || continue
        isempty(ov) && continue
        ts = stats_parse_ts(get(rec, "ts", ""))
        ts === nothing && continue
        append!(events, fill(ts, length(ov)))
    end
    sort!(events)
    cv = Tuple{Dates.DateTime,Int}[]
    for p in series
        ts = stats_parse_ts(get(p, "ts", ""))
        ts === nothing && continue
        push!(cv, (ts, Int(get(p, "c", 0))))
    end
    sort!(cv; by=first)
    dones = Tuple{String,Dates.DateTime}[]
    for n in listnodes(st, :w; include_archived=true)
        for i in ivals[n.id]
            (i.status === :done && i.start !== nothing) || continue
            push!(dones, (n.id, i.start))
        end
    end
    sort!(dones; by=x -> (x[2], x[1]))
    out = Dict{String,Any}[]
    prev = nothing
    for (wid, ts) in dones
        delta = count(t -> (prev === nothing || t > prev) && t <= ts, events)
        c = 0
        for (cts, cc) in cv
            cts <= ts || break
            c = cc
        end
        push!(out, Dict{String,Any}(
            "id" => wid,
            "ts" => Dates.format(ts, "yyyy-mm-ddTHH:MM:SS") * "Z",
            "delta" => delta,
            "c" => c,
        ))
        prev = ts
    end
    out
end

function compute_stats(st::State, recs::Vector{Dict{String,Any}};
                       now_ts::AbstractString=utc_stamp_second())::Dict{String,Any}
    now_dt = something(stats_parse_ts(now_ts))
    ivals = stats_intervals(st, recs, now_dt)
    (cycle_classes, cycle_seconds) = stats_cycle_time(st, ivals)
    (reject_total, reject_per_node, progress_entries, first_pass, first_pass_rate) =
        stats_dor(st, recs, ivals)
    (bet_counts, bet_ratio) = stats_bets(st, ivals)
    (stale_entries, revalidations, gate_runs, gate_empty, overflow_events, invalidated_events, gates) =
        stats_discovery(st, recs, ivals)
    (undo_events, undone_steps, mutations, undos_per_100) = stats_undo(recs)
    surprise_total =
        bet_counts[:invalidated_acceptable] + bet_counts[:invalidated_blocking] + overflow_events
    done_w = count(n -> any(i -> i.status === :done, ivals[n.id]),
                   listnodes(st, :w; include_archived=true))
    (series, replay_failures) = stats_cv_series(st, recs, now_ts)
    (session_entries, session_mean, session_median, session_max) = stats_sessions(recs)
    (latency_dor, latency_discovery) = stats_checkpoint_latency(st, recs, ivals)
    (pai_invalidated, pai_ever, pai_rate) = stats_post_approval_invalidation(st, ivals)
    (dor_split, dor_split_rate) = stats_dor_split(st, recs, ivals)
    (yield_real, yield_null, yield_none, yield_goals) = stats_distill_yield(st, recs)
    surprise_series = stats_surprise_series(st, recs, ivals, series)
    Dict{String,Any}(
        "command" => "stats",
        "records" => length(recs),
        "mutations" => mutations,
        "cycle_time" => Dict{String,Any}(
            "by_cynefin" => cycle_classes,
            "durations_seconds" => cycle_seconds,
        ),
        "dor" => Dict{String,Any}(
            "reject_events" => reject_total,
            "reject_per_node" => reject_per_node,
            "progress_entries" => progress_entries,
            "first_pass" => first_pass,
            "first_pass_rate" => first_pass_rate,
            "first_pass_split" => Dict{String,Any}(
                "no_reject" => dor_split["no_reject"],
                "reject_discovery" => dor_split["reject_discovery"],
                "reject_plain" => dor_split["reject_plain"],
                "discovery_rate" => dor_split_rate,
            ),
        ),
        "bets" => Dict{String,Any}(
            "validated" => bet_counts[:validated],
            "invalidated_acceptable" => bet_counts[:invalidated_acceptable],
            "invalidated_blocking" => bet_counts[:invalidated_blocking],
            "ratio" => bet_ratio,
        ),
        "discovery" => Dict{String,Any}(
            "stale_entries" => stale_entries,
            "revalidations" => revalidations,
            "gate_runs" => gate_runs,
            "gate_empty" => gate_empty,
            "gate_overflow_events" => overflow_events,
            "gate_invalidated_events" => invalidated_events,
        ),
        "gates" => gates,
        "undo" => Dict{String,Any}(
            "undo_events" => undo_events,
            "undone_steps" => undone_steps,
            "undos_per_100_mutations" => undos_per_100,
        ),
        "audit" => Dict{String,Any}(
            "sessions" => Dict{String,Any}(
                "count" => length(session_entries),
                "per_session" => session_entries,
                "mean" => session_mean,
                "median" => session_median,
                "max" => session_max,
            ),
            "checkpoint_latency" => Dict{String,Any}(
                "dor" => stats_hours_summary(latency_dor),
                "discovery" => stats_hours_summary(latency_discovery),
            ),
            "post_approval_invalidation" => Dict{String,Any}(
                "invalidated" => pai_invalidated,
                "ever_validated" => pai_ever,
                "rate" => pai_rate,
            ),
        ),
        "rework" => stats_rework(st, reject_per_node),
        "distill_yield" => Dict{String,Any}(
            "goals_with_real" => yield_real,
            "goals_null_attested" => yield_null,
            "goals_without" => yield_none,
            "goals" => yield_goals,
        ),
        "surprise" => Dict{String,Any}(
            "total" => surprise_total,
            "done_w" => done_w,
            "per_done" => done_w == 0 ? nothing : surprise_total / done_w,
        ),
        "surprise_series" => surprise_series,
        "cv_series" => series,
        "replay_failures" => replay_failures,
    )
end

stats_fmt_num(x)::String = x === nothing ? "–" :
                          x isa AbstractFloat ? string(round(x, digits=2)) : string(x)

function print_stats_human(p::Dict{String,Any})::Nothing
    println("records: ", p["records"])
    println("mutations: ", p["mutations"])
    println()
    println("cycle time (ready -> done):")
    ct = p["cycle_time"]
    classes = sort!(collect(keys(ct["by_cynefin"])))
    if isempty(classes)
        println("  (no W with ready and done intervals)")
    else
        println(@sprintf("  %-10s %5s %8s %9s %8s", "class", "n", "mean h", "median h", "max h"))
        for cls in classes
            d = ct["by_cynefin"][cls]
            println(@sprintf("  %-10s %5d %8s %9s %8s", cls, d["n"],
                stats_fmt_num(d["mean_hours"]), stats_fmt_num(d["median_hours"]),
                stats_fmt_num(d["max_hours"])))
        end
    end
    println()
    dor = p["dor"]
    println("DoR:")
    println("  reject events: ", dor["reject_events"])
    for (id, k) in sort!(collect(dor["reject_per_node"]))
        println("    ", id, ": ", k)
    end
    println("  progress entries: ", dor["progress_entries"])
    println("  first pass: ", dor["first_pass"])
    println("  first pass rate: ", stats_fmt_num(dor["first_pass_rate"]))
    fps = dor["first_pass_split"]
    println("  first pass split:")
    println("    no reject: ", fps["no_reject"])
    println("    reject + discovery: ", fps["reject_discovery"])
    println("    reject plain: ", fps["reject_plain"])
    println("    discovery rate: ", stats_fmt_num(fps["discovery_rate"]))
    println()
    bets = p["bets"]
    println("bets:")
    println("  validated: ", bets["validated"])
    println("  invalidated acceptable: ", bets["invalidated_acceptable"])
    println("  invalidated blocking: ", bets["invalidated_blocking"])
    println("  ratio: ", stats_fmt_num(bets["ratio"]))
    println()
    disc = p["discovery"]
    println("discovery:")
    println("  stale entries: ", disc["stale_entries"])
    println("  revalidations: ", disc["revalidations"])
    println("  gate runs: ", disc["gate_runs"])
    println("  gate empty: ", disc["gate_empty"])
    println("  gate overflow events: ", disc["gate_overflow_events"])
    println("  gate invalidated events: ", disc["gate_invalidated_events"])
    println()
    undo = p["undo"]
    println("undo:")
    println("  events: ", undo["undo_events"])
    println("  undone steps: ", undo["undone_steps"])
    println("  per 100 mutations: ", stats_fmt_num(undo["undos_per_100_mutations"]))
    println()
    audit = p["audit"]
    println("audit:")
    sess = audit["sessions"]
    println("  commands per session:")
    for e in sess["per_session"]
        tok = e["session"]
        length(tok) > 24 && (tok = first(tok, 24))
        println("    ", tok, " ", e["commands"])
    end
    println("  sessions: ", sess["count"], " mean ", stats_fmt_num(sess["mean"]),
            " median ", stats_fmt_num(sess["median"]), " max ", stats_fmt_num(sess["max"]))
    cpl = audit["checkpoint_latency"]
    println("  checkpoint latency (hours):")
    for (label, key) in (("dor reject -> progress", "dor"),
                         ("discovery proposed -> active", "discovery"))
        d = cpl[key]
        println("    ", label, ": n ", d["n"], " mean ", stats_fmt_num(d["mean_hours"]),
                " median ", stats_fmt_num(d["median_hours"]), " max ", stats_fmt_num(d["max_hours"]))
    end
    pai = audit["post_approval_invalidation"]
    println("  post-approval invalidation: ", pai["invalidated"], " / ", pai["ever_validated"],
            " (rate ", stats_fmt_num(pai["rate"]), ")")
    println()
    rework = p["rework"]
    println("rework:")
    for key in ("covered", "uncovered")
        g = rework[key]
        println("  ", key, ": ", g["w"], " W, ", g["rejects"], " rejects, mean ",
                stats_fmt_num(g["mean_rejects"]), " per W")
        for r in g["per_w"]
            println("    ", r["id"], ": ", r["rejects"])
        end
    end
    println("  note: undo events are global; undone journal lines are dropped")
    println()
    dy = p["distill_yield"]
    println("distill yield:")
    println("  real: ", dy["goals_with_real"], " null: ", dy["goals_null_attested"],
            " none: ", dy["goals_without"])
    for e in dy["goals"]
        if e["status"] == "real"
            println("  ", e["goal"], " real ", join(e["discoveries"], " "))
        else
            println("  ", e["goal"], " ", e["status"])
        end
    end
    println()
    surprise = p["surprise"]
    println("surprise:")
    println("  total: ", surprise["total"])
    println("  done W: ", surprise["done_w"])
    println("  per done: ", stats_fmt_num(surprise["per_done"]))
    println()
    println("surprise series:")
    ss = p["surprise_series"]
    if isempty(ss)
        println("  –")
    else
        for e in ss
            println("  ", e["id"], " ", e["ts"], " +", e["delta"], " C=", e["c"])
        end
    end
    println()
    series = p["cv_series"]
    println("cv series: ", length(series), " points (replay failures: ", p["replay_failures"], ")")
    if !isempty(series)
        f = series[1]
        l = series[end]
        println("  first: ", f["ts"], " C=", f["c"], " V=", f["v"])
        println("  last:  ", l["ts"], " C=", l["c"], " V=", l["v"])
    end
    nothing
end
