const EXIT_OK = 0
const EXIT_ERR = 1
const EXIT_CHECKSUM = 2
const EXIT_INVARIANT = 3
const EXIT_GUARD = 4
const EXIT_NOTFOUND = 5

mutable struct CliCtx
    root::String
    quiet::Bool
    json::Bool
    no_render::Bool
end
CliCtx() = CliCtx(pwd(), false, false, false)

devdir(ctx::CliCtx) = joinpath(ctx.root, ".grove")
lockpath(ctx::CliCtx) = joinpath(devdir(ctx), "state.lock")
function grove_version()::String
    for l in eachline(joinpath(@__DIR__, "..", "Project.toml"))
        startswith(l, "version = ") && return strip(strip(l[11:end]), '"')
    end
    return "unknown"
end

const SKILL_PAGES = ("SKILL.md", "specification.md")

skill_src_dir() = joinpath(@__DIR__, "..", "..", "..", "docs", "skills", "grove")
specification_src() = joinpath(@__DIR__, "..", "..", "..", "docs", "specification.md")

function stamp_skill(md::AbstractString)::String
    portable = replace(md, "../../specification.md" => "specification.md")
    lines = split(portable, '\n')
    head = lines[1:min(3, length(lines))]
    any(l -> startswith(l, "version:"), head) && return portable
    ver = grove_version()
    out = IOBuffer()
    for (i, l) in enumerate(lines)
        (i == length(lines) && isempty(l)) && break
        print(out, l, '\n')
        (i == 1 && l == "---") && print(out, "version: ", ver, '\n')
    end
    return String(take!(out))
end

function cmd_skill(ctx::CliCtx, pos, kw)::Int
    dir = get(kw, "install", "")
    srcroot = skill_src_dir()
    if isempty(dir)
        md = stamp_skill(read(joinpath(srcroot, "SKILL.md"), String))
        if ctx.json
            json_cli_out(Dict("command" => "skill", "mode" => "print", "markdown" => md))
            return EXIT_OK
        end
        print(md)
        return EXIT_OK
    end
    target = joinpath(dir, "grove")
    mkpath(target)
    n = 0
    for p in SKILL_PAGES
        text = p == "SKILL.md" ? stamp_skill(read(joinpath(srcroot, p), String)) : read(specification_src(), String)
        write(joinpath(target, p), text)
        n += 1
    end
    if ctx.json
        json_cli_out(Dict("command" => "skill", "mode" => "install", "files" => n, "target" => target))
        return EXIT_OK
    end
    println("installed $n skill files to $target")
    EXIT_OK
end

function cmd_specification(ctx::CliCtx, pos, kw)::Int
    md = read(specification_src(), String)
    if ctx.json
        json_cli_out(Dict("command" => "specification", "mode" => "print", "markdown" => md))
    else
        print(md)
    end
    EXIT_OK
end


indexpath(ctx::CliCtx) = joinpath(devdir(ctx), "index.md")
glossarypath(ctx::CliCtx) = joinpath(devdir(ctx), "glossary.md")

function parse_args(args::Vector{String})::Tuple{CliCtx,Vector{String},Dict{String,String}}
    ctx = CliCtx()
    pos = String[]
    kw = Dict{String,String}()
    for a in args
        if startswith(a, "--")
            eq = findfirst('=', a)
            if eq === nothing
                key = a[3:end]; val = "true"
            else
                key = a[3:eq-1]; val = a[eq+1:end]
            end
            if key == "root"; ctx.root = abspath(val)
            elseif key == "quiet"; ctx.quiet = (val == "true")
            elseif key == "json"; ctx.json = (val == "true")
            elseif key == "no-render"; ctx.no_render = (val == "true")
            else; kw[key] = val
            end
        else
            push!(pos, a)
        end
    end
    ctx, pos, kw
end

function emit_diag(ctx::CliCtx, d::Diagnostic)
    (ctx.quiet && d.severity == "info") && return
    println(stderr, render_diag(d))
end

function fail_diag(ctx::CliCtx, exitcode::Int, d::Diagnostic)::Int
    emit_diag(ctx, d)
    exitcode
end

ferr(ctx::CliCtx, exitcode::Int, text::AbstractString)::Int =
    fail_diag(ctx, exitcode, interface_error(text))
finv(ctx::CliCtx, exitcode::Int, text::AbstractString)::Int =
    fail_diag(ctx, exitcode, invariant_error(text))
fsys(ctx::CliCtx, exitcode::Int, text::AbstractString)::Int =
    fail_diag(ctx, exitcode, system_error(text))

function fusage(ctx::CliCtx, cmd::AbstractString, usage::AbstractString)::Int
    println(stderr, render_diag(interface_error("invalid arguments for $(cmd)")))
    println(stderr, usage)
    EXIT_ERR
end

struct CliFailure <: Exception
    code::Int
    diag::Diagnostic
end

function load(ctx::CliCtx; verify::Bool=true)::State
    p = lockpath(ctx)
    isfile(p) || throw(CliFailure(EXIT_ERR, system_error(
        "lock file is absent, initialize the project first",
        Pair{String,String}["path" => p])))
    try
        return read_lock(p; verify=verify)
    catch e
        if e isa ChecksumMismatch
            throw(CliFailure(EXIT_CHECKSUM, system_error(
                "lock checksum mismatch, the file may have been edited by hand, accept the current contents with a repair",
                Pair{String,String}["expected" => e.expected, "actual" => e.actual])))
        end
        rethrow()
    end
end

function dashboard_decay_count(ctx::CliCtx, st::State)::Int
    any(x -> x.status in (:proposed, :active), listnodes(st, :y)) || return 0
    errs = discovery_decay_errors(st, ctx.root, glossarypath(ctx))
    ids = Set{String}()
    for e in errs
        toks = filter(s -> !isempty(s), split(e.text, ' '))
        isempty(toks) || push!(ids, String(toks[1]))
    end
    length(ids)
end

function journal_session_token(ctx::CliCtx, kw::AbstractDict)::String
    t = strip(effective_session_token(ctx.root, kw))
    isempty(t) ? "none" : t
end

function persist(ctx::CliCtx, st::State; journal::Union{Nothing,AbstractDict}=nothing, session=nothing)
    rederive_artifacts!(st)
    write_lock(lockpath(ctx), st)
    ctx.no_render || write_index(indexpath(ctx), st; decay=dashboard_decay_count(ctx, st))
    if journal !== nothing
        if !haskey(journal, "session")
            t = session === nothing ? journal_session_token(ctx, Dict{String,String}()) :
                strip(String(session))
            journal["session"] = isempty(t) ? "none" : t
        end
        append_journal_record!(journalpath(ctx), journal)
    end
end

function cmd_init(ctx::CliCtx, pos, kw)
    isfile(lockpath(ctx)) && (return fail_diag(ctx, EXIT_ERR,
        system_error("lock already exists", Pair{String,String}["path" => lockpath(ctx)])))
    isdir(devdir(ctx)) || mkpath(devdir(ctx))
    st = State()
    if haskey(kw, "id-stride")
        v = tryparse(Int, kw["id-stride"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --id-stride (expected integer)"))
        v < 1 && (return ferr(ctx, EXIT_ERR, "--id-stride must be ≥ 1"))
        st.id_stride = Int(v)
    end
    if haskey(kw, "id-offset")
        v = tryparse(Int, kw["id-offset"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --id-offset (expected integer)"))
        v < 1 && (return ferr(ctx, EXIT_ERR, "--id-offset must be ≥ 1"))
        st.id_offset = Int(v)
    end
    if haskey(kw, "id-width")
        w = tryparse(Int, kw["id-width"])
        w === nothing && (return ferr(ctx, EXIT_ERR, "bad --id-width (expected integer)"))
        w < 2 && (return ferr(ctx, EXIT_ERR, "--id-width must be ≥ 2"))
        st.id_pad_width = Int(w)
    elseif st.id_stride != 1 || st.id_offset != 1
        st.id_pad_width = max(st.id_pad_width, 3)
    end
    persist(ctx, st)
    isfile(glossarypath(ctx)) || open(glossarypath(ctx), "w") do io
        println(io, "# Glossary")
        println(io)
        println(io, "| Term | Definition | Source |")
        println(io, "| --- | --- | --- |")
    end
    emit_diag(ctx, operational_info("initialised", Pair{String,String}["path" => devdir(ctx)]))
    EXIT_OK
end

function csv_dup_guard(ctx::CliCtx, kind::Symbol, opt::AbstractString, entries::Vector{String})::Union{Nothing,Int}
    seen = Set{String}()
    for e in entries
        s = strip(e)
        if s in seen
            fail_diag(ctx, EXIT_ERR, interface_error("add $(kind), --$(opt) has duplicate entry \"$s\""))
            return EXIT_ERR
        end
        push!(seen, s)
    end
    nothing
end

const USAGE_ADD = "usage: grove add <kind> --title=\"…\" [...]"

function cmd_add(ctx::CliCtx, pos, kw)
    haskey(kw, "help") && (println(USAGE_ADD); return EXIT_OK)
    length(pos) >= 1 || (return fusage(ctx, "add", USAGE_ADD))
    length(pos) > 1 && (return ferr(ctx, EXIT_ERR, "add received unexpected positional argument $(pos[2])"))
    isempty(kw) && !(pos[1] in ("a", "y")) && (return fusage(ctx, "add", USAGE_ADD))
    kind = Symbol(pos[1])
    kind in NODE_KINDS || (return ferr(ctx, EXIT_ERR, "unknown kind $kind"))
    st = load(ctx)
    id = next_id!(st, kind)
    n = Node(kind, id)
    n.title = get(kw, "title", "")
    if kind === :w
        n.type = Symbol(get(kw, "type", "feature"))
        n.cynefin = Symbol(get(kw, "cynefin", "complicated"))
        n.status = Symbol(get(kw, "status", "proposed"))
        if haskey(kw, "goals")
            goals = String.(split(kw["goals"], ","))
            rc = csv_dup_guard(ctx, kind, "goals", goals)
            rc !== nothing && return rc
            n.fields[:goals] = goals
        end
        haskey(kw, "theme") && (n.fields[:theme] = kw["theme"])
        surface = String[String(s) for s in split(get(kw, "surface", ""), ',') if !isempty(strip(s))]
        rc = csv_dup_guard(ctx, kind, "surface", surface)
        rc !== nothing && return rc
        isempty(surface) || (n.fields[:surface] = surface)
    elseif kind === :g
        n.status = Symbol(get(kw, "status", "unverified"))
        haskey(kw, "fitness") &&
            (return ferr(ctx, EXIT_ERR, "add g, --fitness is retired, use --fitness-kind and --fitness-target"))
        if haskey(kw, "fitness-kind")
            fk = Symbol(lowercase(strip(kw["fitness-kind"])))
            fk in GOAL_FITNESS_KINDS || (return ferr(ctx, EXIT_ERR, "bad --fitness-kind"))
            n.attrs["fitness_kind"] = String(fk)
        end
        haskey(kw, "fitness-target") && (n.fields[:fitness_target] = kw["fitness-target"])
        aref = strip(get(kw, "area", ""))
        isempty(aref) && (return ferr(ctx, EXIT_ERR, "add g, --area=A-NN is required, create an area node first"))
        an = get(st.nodes, aref, nothing)
        (an === nothing || an.kind !== :a) && (return ferr(ctx, EXIT_ERR, "add g, unknown --area id $aref"))
        n.fields[:area] = aref
        has_fitness = haskey(kw, "fitness-kind")
        has_fitness || (return ferr(ctx, EXIT_ERR, "add g, fitness is required, pass --fitness-kind and --fitness-target or --fitness-kind=manual"))
        if haskey(kw, "fitness-kind")
            fk2 = lowercase(strip(kw["fitness-kind"]))
            if fk2 in ("count", "metric", "ratio")
                isempty(strip(get(kw, "fitness-target", ""))) &&
                    (return ferr(ctx, EXIT_ERR, "add g, --fitness-target is required for --fitness-kind=$fk2"))
            end
        end
        if n.status === :verified && goal_structured_kind(n) !== :manual
            fail_diag(ctx, EXIT_GUARD, invariant_error("add g, --status=verified is derived from fitness deltas"))
            return EXIT_GUARD
        end
    elseif kind === :d
        n.status = Symbol(get(kw, "status", "proposed"))
    elseif kind === :q
        n.status = Symbol(get(kw, "status", "open"))
        n.cynefin = Symbol(get(kw, "cynefin", "complicated"))
    elseif kind === :b
        n.status = Symbol(get(kw, "status", "proposed"))
        n.cynefin = Symbol(get(kw, "cynefin", "complicated"))
    elseif kind === :t
        n.status = Symbol(get(kw, "status", "open"))
    elseif kind === :y
        n.status = :proposed
        isempty(strip(n.title)) && (return ferr(ctx, EXIT_ERR, "add y, --title is required"))
        tags = String[String(t) for t in split(get(kw, "tags", ""), ',') if !isempty(strip(t))]
        isempty(tags) && (return ferr(ctx, EXIT_ERR, "add y, --tags requires at least one glossary term"))
        rc = csv_dup_guard(ctx, kind, "tags", tags)
        rc !== nothing && return rc
        n.fields[:tags] = tags
        surface = String[String(s) for s in split(get(kw, "surface", ""), ',') if !isempty(strip(s))]
        rc = csv_dup_guard(ctx, kind, "surface", surface)
        rc !== nothing && return rc
        isempty(surface) || (n.fields[:surface] = surface)
        haskey(kw, "why") && (n.fields[:why] = String[kw["why"]])
        if !haskey(n.fields, :surface)
            ok_why = haskey(n.fields, :why) && prose_field_nonempty(n.fields[:why])
            ok_why || (return ferr(ctx, EXIT_ERR, "add y, --surface absent requires --why prose"))
        end
        from = String[String(t) for t in split(get(kw, "from", ""), ',') if !isempty(strip(t))]
        isempty(from) && (return ferr(ctx, EXIT_ERR, "add y, --from requires at least one provenance record"))
    elseif kind === :a
        n.status = :present
        isempty(strip(n.title)) && (return ferr(ctx, EXIT_ERR, "add a, --title is required"))
        surface = String[String(s) for s in split(get(kw, "surface", ""), ',') if !isempty(strip(s))]
        rc = csv_dup_guard(ctx, kind, "surface", surface)
        rc !== nothing && return rc
        isempty(surface) || (n.fields[:surface] = surface)
    end
    st.nodes[id] = n
    stamp_new_node!(n)

    rc = flush_add_edges!(ctx, kind, id, kw, st)
    rc !== nothing && return rc

    persist(ctx, st; journal=wrap_journal_record("add", journal_inverse_rm_node(id)),
            session=journal_session_token(ctx, kw))
    if ctx.json
        json_cli_out(Dict{String,Any}("command" => "add", "kind" => String(kind), "id" => id))
    else
        println(id)
    end
    EXIT_OK
end

function flush_add_edges!(ctx::CliCtx, kind::Symbol, id::AbstractString, kw::Dict{String,String},
                          st::State)::Union{Nothing,Int}
    if kind === :d && haskey(kw, "supersedes")
        for oid in split(kw["supersedes"], ',')
            oid = strip(oid); isempty(oid) && continue
            r = validate_and_push_edge!(st, id, :supersedes, oid)
            if r !== nothing
                fail_diag(ctx, EXIT_GUARD, r); return EXIT_GUARD
            end
        end
    elseif kind === :q && haskey(kw, "targets")
        for tid in split(kw["targets"], ',')
            tid = strip(tid); isempty(tid) && continue
            r = validate_and_push_edge!(st, id, :asks, tid)
            if r !== nothing
                fail_diag(ctx, EXIT_GUARD, r); return EXIT_GUARD
            end
        end
    elseif kind === :b
        if haskey(kw, "tests")
            for qid in split(kw["tests"], ',')
                qid = strip(qid); isempty(qid) && continue
                r = validate_and_push_edge!(st, id, :tests, qid)
                if r !== nothing
                    fail_diag(ctx, EXIT_GUARD, r); return EXIT_GUARD
                end
            end
        end
        if haskey(kw, "targets")
            for wid in split(kw["targets"], ',')
                wid = strip(wid); isempty(wid) && continue
                r = validate_and_push_edge!(st, id, :targets, wid)
                if r !== nothing
                    fail_diag(ctx, EXIT_GUARD, r); return EXIT_GUARD
                end
            end
        end
    elseif kind === :y && haskey(kw, "from")
        for oid in split(kw["from"], ',')
            oid = strip(oid); isempty(oid) && continue
            src = get(st.nodes, oid, nothing)
            src === nothing && return ferr(ctx, EXIT_GUARD, "add y, unknown --from id $oid")
            r = if src.kind === :w
                validate_and_push_edge!(st, oid, :produces, id)
            elseif src.kind in (:d, :q, :b)
                validate_and_push_edge!(st, id, :distills, oid)
            else
                interface_error("add y, --from $oid must reference W or D, Q or B")
            end
            if r !== nothing
                fail_diag(ctx, EXIT_GUARD, r); return EXIT_GUARD
            end
        end
    end
    nothing
end

function goal_notes_distill_deferred(g::Node)::Bool
    g.kind === :g || return false
    any(ln -> occursin("--distill-deferred", String(ln)), get(g.fields, :notes, String[]))
end

function print_lazy_distill_prompt_on_newly_verified_goals!(
    ctx::CliCtx, st::State, w::Node, old_goal_status::Dict{String,String},
)::Nothing
    w.kind === :w || return nothing
    for gid_raw in get(w.fields, :goals, String[])
        gid = String(strip(string(gid_raw)))
        isempty(gid) && continue
        ost = get(old_goal_status, gid, nothing)
        ost === nothing && continue
        Symbol(ost) === :verified && continue
        g = get(st.nodes, gid, nothing)
        (g === nothing || g.kind !== :g) && continue
        g.status !== :verified && continue
        goal_notes_distill_deferred(g) && continue
        emit_diag(ctx, operational_info(
            "goal $(gid) ($(g.title)) is verified, distill its content or attest a null distillation, or add a notes line containing --distill-deferred to skip"))
    end
    nothing
end

function json_cli_out(obj::AbstractDict)::Nothing
    JSON.print(stdout, obj)
    println()
    nothing
end

function json_field_value(kind::Symbol, fname::Symbol, v::Any)::Any
    form = FIELD_CATALOG[(kind, fname)]
    if form === :prose || form === :reflist
        v === nothing && return String[]
        return String[v...]
    elseif form === :single
        return v === nothing ? "" : string(v)
    elseif form === :fitness
        v === nothing && return Dict{String,Int}()
        return Dict{String,Int}(v)
    end
    nothing
end

function json_node_snapshot(n::Node)::Dict{String,Any}
    fields = Dict{String,Any}()
    for fname in FIELD_ORDER[n.kind]
        haskey(n.fields, fname) || continue
        fields[string(fname)] = json_field_value(n.kind, fname, n.fields[fname])
    end
    d = Dict{String,Any}(
        "command" => "show",
        "record" => Dict{String,Any}(
            "kind" => string(n.kind),
            "id" => n.id,
            "title" => n.title,
            "status" => string(n.status),
            "archived" => n.archived,
            "attrs" => Dict{String,String}(string(k) => string(v) for (k, v) in n.attrs),
            "fields" => fields,
        ),
    )
    if n.type !== nothing
        d["record"]["type"] = string(n.type)
    end
    if n.cynefin !== nothing
        d["record"]["cynefin"] = string(n.cynefin)
    end
    d
end

function cmd_set(ctx::CliCtx, pos, kw)
    length(pos) >= 2 || (return ferr(ctx, EXIT_ERR, "set requires an id and key=value"))
    id = pos[1]
    eq = findfirst('=', pos[2])
    eq === nothing && (return ferr(ctx, EXIT_ERR, "expected key=value"))
    key = pos[2][1:eq-1]
    val = pos[2][eq+1:end]
    st = load(ctx)
    n = get(st.nodes, id, nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    eff = effective_session_token(ctx.root, kw)
    jr = nothing

    if key == "status"
        new_status = Symbol(val)
        if n.kind === :w && n.status === :progress && new_status !== :progress
            msg = session_denial_progress_release(n, eff)
            msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
        end
        rc = guard_status_transition(ctx, st, n, new_status, kw)
        rc != EXIT_OK && return rc
        old_status = n.status
        if n.kind === :w
            gs = Dict{String,String}()
            for gid in get(n.fields, :goals, String[])
                g = get(st.nodes, gid, nothing)
                g === nothing && continue
                gs[gid] = string(g.status)
            end
            inv = Dict{String,Any}(
                "op" => "set_w_status_with_goals",
                "id" => String(id),
                "old_w_status" => string(old_status),
                "goal_statuses" => gs,
            )
            merge!(inv, session_journal_snap(n))
            jr = wrap_journal_record("set", inv)
        else
            jr = wrap_journal_record("set", Dict{String,Any}(
                "op" => "set_status_plain",
                "id" => String(id),
                "old_status" => string(n.status),
            ))
        end
        n.status = new_status
        if n.kind === :w
            if new_status === :progress
                assign_w_claim_session!(n, eff)
            elseif old_status === :progress && new_status !== :progress
                clear_w_session_attrs!(n)
            end
            rederive_goals!(st, n)
            new_status === :done && print_lazy_distill_prompt_on_newly_verified_goals!(ctx, st, n, gs)
        end
        stamp_touch_node!(n)
        persist(ctx, st; journal=jr, session=eff)
        return EXIT_OK
    end

    if n.kind === :w && n.status === :progress
        msg = session_denial_progress_mutate(n, eff)
        msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    end
    set_key_supported(n.kind, key) ||
        (return ferr(ctx, EXIT_ERR, "unsupported key $key"))
    msg = terminal_freeze_msg(n)
    msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))

    if key == "cynefin"
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_cynefin",
            "id" => String(id),
            "old" => n.cynefin === nothing ? "" : string(n.cynefin),
        ))
        n.cynefin = Symbol(val)
    elseif key == "type"
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_type",
            "id" => String(id),
            "old" => n.type === nothing ? "" : string(n.type),
        ))
        n.type = Symbol(val)
    elseif key == "title"
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_title",
            "id" => String(id),
            "old" => n.title,
        ))
        n.title = val
    elseif key == "fitness" && n.kind === :g
        isempty(strip(val)) ||
            (return ferr(ctx, EXIT_ERR, "set fitness is retired, use fitness_kind and fitness_target or an empty value to remove a legacy label"))
        hb = haskey(n.attrs, "fitness")
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_g_attr_fitness",
            "id" => String(id),
            "had_before" => hb,
            "old" => hb ? String(n.attrs["fitness"]) : "",
        ))
        delete!(n.attrs, "fitness")
        refresh_goal_structured_fitness!(st, n)
    elseif key == "fitness_target" && n.kind === :g
        hb = haskey(n.fields, :fitness_target)
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_g_fitness_target",
            "id" => String(id),
            "had_before" => hb,
            "old" => hb ? String(n.fields[:fitness_target]) : "",
        ))
        if isempty(strip(val))
            delete!(n.fields, :fitness_target)
        else
            n.fields[:fitness_target] = val
        end
        refresh_goal_structured_fitness!(st, n)
    elseif key == "fitness_kind" && n.kind === :g
        ks = Symbol(lowercase(strip(val)))
        ks in GOAL_FITNESS_KINDS || begin
                return ferr(ctx, EXIT_ERR,
                    "bad fitness_kind, expected one of $(join(string.(GOAL_FITNESS_KINDS), ", "))")
            end
        hb = haskey(n.attrs, "fitness_kind")
        oldk = hb ? String(n.attrs["fitness_kind"]) : ""
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_g_attr_fitness_kind",
            "id" => String(id),
            "had_before" => hb,
            "old" => oldk,
            "new" => String(ks),
        ))
        n.attrs["fitness_kind"] = String(ks)
        refresh_goal_structured_fitness!(st, n)
    elseif key == "area" && n.kind === :g
        aref = strip(val)
        an = get(st.nodes, aref, nothing)
        (an === nothing || an.kind !== :a) &&
            (return ferr(ctx, EXIT_ERR, "set unknown area $aref, expected an existing A-NN node"))
        hb = haskey(n.fields, :area)
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_g_area",
            "id" => String(id),
            "had_before" => hb,
            "old" => hb ? String(n.fields[:area]) : "",
        ))
        n.fields[:area] = aref
    elseif key == "requires_coverage" && n.kind in (:g, :t)
        θ = parse_requires_coverage(val)
        θ === nothing &&
            (return ferr(ctx, EXIT_ERR, "bad requires_coverage (expected `true` or a float in (0,1])"))
        hb = haskey(n.attrs, "requires_coverage")
        jr = wrap_journal_record("set", Dict{String,Any}(
            "op" => "set_requires_coverage",
            "id" => String(id),
            "had_before" => hb,
            "old" => hb ? String(n.attrs["requires_coverage"]) : "",
        ))
        n.attrs["requires_coverage"] = val
    else
        fail_diag(ctx, EXIT_ERR, interface_error("unsupported key $key")); return EXIT_ERR
    end
    stamp_touch_node!(n)
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

set_key_supported(kind::Symbol, key::AbstractString)::Bool =
    key in ("cynefin", "type", "title") ||
    (kind === :g && key in ("fitness", "fitness_target", "fitness_kind", "area")) ||
    (kind in (:g, :t) && key == "requires_coverage")

accepted_decision_msg(id::AbstractString)::Diagnostic =
    invariant_error("decision $(id) is accepted, create a successor D with --supersedes")

function terminal_freeze_msg(n::Node)::Union{Nothing,Diagnostic}
    isterminal(n.kind, n.status) || return nothing
    n.kind === :d && n.status === :accepted && return accepted_decision_msg(String(n.id))
    invariant_error("$(n.id) is terminal ($(n.status)), create a successor node or undo the last mutation")
end

function guard_status_transition(ctx::CliCtx, st::State, n::Node, new::Symbol, kw)::Int
    valid = STATUS[n.kind]
    new in valid || return finv(ctx, EXIT_ERR, "invalid status `$(new)` for $(n.kind)")
    if n.kind !== :d || n.status !== :accepted || new !== :superseded
        msg = terminal_freeze_msg(n)
        msg !== nothing && return fail_diag(ctx, EXIT_GUARD, msg)
    end
    if n.kind === :t
        return finv(ctx, EXIT_GUARD, "theme status is derived and cannot be set manually")
    end
    if n.kind === :a
        return finv(ctx, EXIT_GUARD, "area status is structural and cannot be set")
    end
    if n.kind === :g && new === :verified && goal_structured_kind(n) !== :manual
        return finv(ctx, EXIT_GUARD, "goal $(n.id) status is derived from fitness deltas")
    end
    if n.kind === :w && new === :ready && n.type === :feature
        bad = filter(b -> begin
            bn = get(st.nodes, b, nothing)
            bn === nothing || !(bn.status === :validated || bn.status === :invalidated_acceptable)
        end, bchain(st, n))
        if !isempty(bad)
            detail = join(map(b -> begin
                bn = get(st.nodes, b, nothing)
                "$b ($(bn === nothing ? "missing" : bn.status))"
            end, bad), ", ")
            return fail_diag(ctx, EXIT_GUARD, invariant_error(
                "$(n.id) cannot enter ready, the assumption chain must be validated or invalidated_acceptable",
                Pair{String,String}["chain" => detail]))
        end
    end
    if n.kind === :w && new === :progress
        if !dor(st, n)
            append_journal_record!(journalpath(ctx), wrap_journal_record("set", Dict{String,Any}(
                "op" => "dor_reject",
                "id" => String(n.id),
                "missing" => [String(label) for (label, ok, _) in dor_breakdown(st, n) if !ok],
            ); session=journal_session_token(ctx, kw)))
            return fail_diag(ctx, EXIT_GUARD,
                invariant_error("DoR is not satisfied for $(n.id), inspect readiness for the failing conjuncts"))
        end
        preds_clear(st, n.id) || return fail_diag(ctx, EXIT_GUARD, invariant_error(
            "predecessors not cleared (goal blockers must be verified, not merely declined, partial, or unverified)"))
        wip = count(w -> w.status === :progress, listnodes(st, :w))
        wip >= WIP_LIMIT_DEFAULT && return fail_diag(ctx, EXIT_GUARD, invariant_error(
            "WIP limit reached", Pair{String,String}["limit" => string(WIP_LIMIT_DEFAULT)]))
    end
    if n.kind === :w && new === :done
        preds_clear(st, n.id) || return fail_diag(ctx, EXIT_GUARD, invariant_error(
            "predecessors not cleared (goal blockers must be verified, not merely declined, partial, or unverified)"))
        if n.type === :spike
            has_produces = any(e -> e.label === :produces && e.from == n.id, st.edges)
            has_produces || return finv(ctx, EXIT_GUARD,
                "$(n.id) is a done spike but produces is empty (no outgoing produces edges)")
        end
        ev = get(n.fields, :evidence, String[])
        isempty(ev) && return finv(ctx, EXIT_GUARD, "$(n.id) has no evidence, record evidence for the item")
        gs = get(n.fields, :goals, String[])
        f = get(n.fields, :fitness, Dict{String,Int}())
        for g in gs
            haskey(f, g) || return fail_diag(ctx, EXIT_GUARD, invariant_error(
                "missing fitness delta for $(g), stage a delta for every linked goal",
                Pair{String,String}["goal" => String(g), "work" => String(n.id)]))
        end
    end
    if n.kind === :y
        cur = n.status
        ok = if new === :superseded
            cur !== :superseded
        elseif cur === :proposed && new === :active
            issues = discovery_anchor_issues(st, n)
            if !isempty(issues)
                details = Pair{String,String}[]
                for (i, issue) in enumerate(issues)
                    push!(details, "issue_$(i)" => issue.text)
                end
                return fail_diag(ctx, EXIT_GUARD, invariant_error(
                    "anchors not satisfied for $(n.id), activation from proposed is refused", details))
            end
            true
        elseif cur === :active && new === :stale
            true
        else
            false
        end
        ok || return fail_diag(ctx, EXIT_GUARD, invariant_error(
            "illegal discovery transition $(cur) to $(new)",
            Pair{String,String}["allowed" => "proposed→active, active→stale, non-terminal→superseded; stale→active only through revalidate"]))
    end
    EXIT_OK
end

function cmd_field(ctx::CliCtx, pos, kw)
    length(pos) >= 3 || (return ferr(ctx, EXIT_ERR, "field requires an id, field, and operation"))
    id, fname, op = pos[1], Symbol(pos[2]), pos[3]
    st = load(ctx)
    n = get(st.nodes, id, nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    eff = effective_session_token(ctx.root, kw)
    if n.kind === :w && n.status === :progress
        msg = session_denial_progress_mutate(n, eff)
        msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    end
    form = get(FIELD_CATALOG, (n.kind, fname), nothing)
    form === nothing && (return ferr(ctx, EXIT_ERR, "unknown field $fname on $(n.kind)"))
    msg = terminal_freeze_msg(n)
    msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    if n.kind === :g && fname === :fitness_current
        kg = goal_structured_kind(n)
        if kg !== nothing && kg !== :manual
            fail_diag(ctx, EXIT_GUARD,
                invariant_error("fitness_current is derived for structured goals, set fitness_kind to manual to author it"))
            return EXIT_GUARD
        end
    end
    jr::Union{Nothing,Dict{String,Any}} = nothing
    if op == "clear"
        if form === :prose || form === :reflist
            oldv = String.(get_vector_field!(n, fname))
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_restore_lines",
                "id" => String(id),
                "field" => String(fname),
                "lines" => oldv,
            ))
            n.fields[fname] = String[]
        elseif form === :fitness
            oldd = Dict{String,Any}(k => Int(v) for (k, v) in copy(get!(n.fields, fname, Dict{String,Int}())))
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_restore_fitness",
                "id" => String(id),
                "field" => String(fname),
                "map" => oldd,
            ))
            n.fields[fname] = Dict{String,Int}()
        elseif form === :single
            prev = haskey(n.fields, fname) ? string(n.fields[fname]) : ""
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_restore_single",
                "id" => String(id),
                "field" => String(fname),
                "value" => prev,
            ))
            n.fields[fname] = ""
        end
    elseif op == "add"
        length(pos) >= 4 || (return ferr(ctx, EXIT_ERR, "missing value"))
        val = pos[4]
        if form === :prose
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_pop_last",
                "id" => String(id),
                "field" => String(fname),
            ))
            push!(get!(n.fields, fname, String[]), val)
        elseif form === :reflist
            if val in get(n.fields, fname, String[])
                fail_diag(ctx, EXIT_GUARD, invariant_error("$(id) $(fname) already contains \"$(val)\""))
                return EXIT_GUARD
            end
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_pop_last",
                "id" => String(id),
                "field" => String(fname),
            ))
            push!(get!(n.fields, fname, String[]), val)
        elseif form === :single
            prev = haskey(n.fields, fname) ? string(n.fields[fname]) : ""
            jr = wrap_journal_record("field", Dict{String,Any}(
                "op" => "field_restore_single",
                "id" => String(id),
                "field" => String(fname),
                "value" => prev,
            ))
            n.fields[fname] = val
        else
            (return ferr(ctx, EXIT_ERR, "field $fname not addable"))
        end
    elseif op == "rm"
        length(pos) >= 4 || (return ferr(ctx, EXIT_ERR, "missing index"))
        idx = tryparse(Int, pos[4])
        idx === nothing && (return ferr(ctx, EXIT_ERR, "index must be an integer"))
        v = String.(copy(get_vector_field!(n, fname)))
        (idx < 1 || idx > length(v)) && (return ferr(ctx, EXIT_ERR, "index out of range"))
        removed = v[idx]
        jr = wrap_journal_record("field", Dict{String,Any}(
            "op" => "field_insert_line",
            "id" => String(id),
            "field" => String(fname),
            "index" => idx,
            "line" => removed,
        ))
        deleteat!(n.fields[fname], idx)
    else
        (return ferr(ctx, EXIT_ERR, "unknown op $op"))
    end
    if n.kind === :g && fname === :fitness_target && op in ("add", "clear")
        refresh_goal_structured_fitness!(st, n)
    end
    stamp_touch_node!(n)
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function guard_sessions_for_progress_endpoints!(ctx::CliCtx, st::State, kw,
                                                from::AbstractString, to::AbstractString)::Int
    eff = effective_session_token(ctx.root, kw)
    for id in (from, to)
        n = get(st.nodes, id, nothing)
        n === nothing && continue
        n.kind !== :w && continue
        n.status !== :progress && continue
        msg = session_denial_progress_mutate(n, eff)
        msg !== nothing && return fail_diag(ctx, EXIT_GUARD, msg)
    end
    EXIT_OK
end

function cmd_link(ctx::CliCtx, pos, kw)
    length(pos) >= 3 || (return ferr(ctx, EXIT_ERR, "link requires a source, label, and target"))
    from, label, to = pos[1], Symbol(pos[2]), pos[3]
    label in EDGE_LABELS || (return ferr(ctx, EXIT_ERR, "unknown label $label"))
    st = load(ctx)
    rc = guard_sessions_for_progress_endpoints!(ctx, st, kw, from, to)
    rc != EXIT_OK && return rc
    r = validate_and_push_edge!(st, from, label, to)
    if r !== nothing
        fail_diag(ctx, EXIT_GUARD, r)
        return EXIT_GUARD
    end
    jr = wrap_journal_record("link", journal_inverse_of_link_forward(label, from, to))
    persist(ctx, st; journal=jr, session=journal_session_token(ctx, kw))
    EXIT_OK
end

function cmd_unlink(ctx::CliCtx, pos, kw)
    length(pos) >= 3 || (return ferr(ctx, EXIT_ERR, "unlink requires a source, label, and target"))
    from, label, to = pos[1], Symbol(pos[2]), pos[3]
    st = load(ctx)
    ee = nothing
    for e in st.edges
        if e.from == from && e.label === label && e.to == to
            ee = e
            break
        end
    end
    ee === nothing && (return ferr(ctx, EXIT_NOTFOUND, "no such edge"))
    rc = guard_sessions_for_progress_endpoints!(ctx, st, kw, from, to)
    rc != EXIT_OK && return rc
    jr = wrap_journal_record("unlink", journal_inverse_restore_edge(from, label, to, ee.t_created))
    filter!(e -> !(e.from == from && e.label === label && e.to == to), st.edges)
    haskey(st.nodes, from) && stamp_touch_node!(st.nodes[from])
    haskey(st.nodes, to) && stamp_touch_node!(st.nodes[to])
    persist(ctx, st; journal=jr, session=journal_session_token(ctx, kw))
    EXIT_OK
end

function cmd_evidence(ctx::CliCtx, pos, kw)
    length(pos) >= 2 || (return ferr(ctx, EXIT_ERR, "evidence requires a work id and text"))
    cmd_field(ctx, [pos[1], "evidence", "add", pos[2]], kw)
end

function cmd_fitness(ctx::CliCtx, pos, kw)
    length(pos) >= 3 || (return ferr(ctx, EXIT_ERR, "fitness requires a work id, goal id, and delta"))
    wid, gid = pos[1], pos[2]
    delta = tryparse(Int, pos[3])
    delta === nothing && (return ferr(ctx, EXIT_ERR, "bad delta (expected integer)"))
    st = load(ctx)
    w = get(st.nodes, wid, nothing); w === nothing && (return ferr(ctx, EXIT_NOTFOUND, "missing $wid"))
    haskey(st.nodes, gid) || (return ferr(ctx, EXIT_NOTFOUND, "missing $gid"))
    msg = terminal_freeze_msg(w)
    msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    eff = effective_session_token(ctx.root, kw)
    msg = session_denial_progress_mutate(w, eff)
    msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    f = get!(w.fields, :fitness, Dict{String,Int}())
    had_key = haskey(f, gid)
    previous = had_key ? f[gid] : nothing
    f[gid] = delta
    stamp_touch_node!(w)
    jr = wrap_journal_record("fitness", journal_inverse_restore_fitness_key(wid, gid, had_key, previous))
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function cmd_archive(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "archive", "usage: grove archive <G-NN>"))
    gid = pos[1]
    st = load(ctx)
    g = get(st.nodes, gid, nothing)
    g === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $gid"))
    g.status === :verified || (return finv(ctx, EXIT_GUARD, "goal must be verified before archiving"))
    ids = exclusive_archive_ids(st, gid)
    distilled = !isempty(distill_linked_da_ids(st, ids)) || distill_null_attested(journalpath(ctx), gid)
    distilled ||
        (return finv(ctx, EXIT_GUARD, "archive $gid requires distillation first, distill the goal or attest a null distillation"))
    eff = effective_session_token(ctx.root, kw)
    orphans = archive_orphan_ids(st, gid)
    for w in listnodes(st, :w)
        citing = gid in get(w.fields, :goals, String[])
        (citing || w.id in orphans) || continue
        w.status !== :progress && continue
        msg = session_denial_progress_mutate(w, eff)
        msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    end
    for id in ids
        n = st.nodes[id]
        n.archived = true
        stamp_touch_node!(n)
    end
    setdiff!(orphans, ids)
    for id in orphans
        haskey(st.nodes, id) || continue
        n = st.nodes[id]
        n.archived = true
        stamp_touch_node!(n)
    end
    all_ids = sort!(collect(union(ids, orphans)); by = id_key)
    persist(ctx, st; session=eff, journal=wrap_journal_record("archive", Dict{String,Any}(
        "op" => JOURNAL_ARCHIVE_OP,
        "id" => String(gid),
        "ids" => String[String(i) for i in all_ids],
    )))
    EXIT_OK
end

function distill_skeleton(id::AbstractString)::String
    "create a discovery from $id with a title, at least one glossary tag, and a surface path or why prose"
end

function distill_candidates(st::State, pool::Set{String})::Vector{Tuple{String,String,String}}
    out = Tuple{String,String,String}[]
    for id in sort!(collect(pool); by = id_key)
        n = get(st.nodes, id, nothing)
        n === nothing && continue
        n.archived && continue
        ok = (n.kind === :b && n.status === :validated) ||
             (n.kind === :q && n.status === :answered) ||
             (n.kind === :d && n.status === :accepted)
        ok || continue
        push!(out, (id, string(n.kind), n.title))
    end
    out
end

function cmd_distill(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "distill", "usage: grove distill <G-NN> [--null]"))
    gid = pos[1]
    st = load(ctx)
    g = get(st.nodes, gid, nothing)
    g === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $gid"))
    g.kind === :g || (return ferr(ctx, EXIT_ERR, "$gid is not a goal"))
    g.status === :verified ||
        (return finv(ctx, EXIT_GUARD, "$gid is $(g.status), distillation happens at verified"))
    mass = exclusive_archive_ids(st, gid)
    linked = distill_linked_da_ids(st, mass)
    attested = distill_null_attested(journalpath(ctx), gid)
    if haskey(kw, "null")
        jr = wrap_journal_record("distill", Dict{String,Any}(
            "op" => JOURNAL_DISTILL_OP,
            "goal" => String(gid),
            "empty" => true,
        ); session=journal_session_token(ctx, kw))
        append_journal_record!(journalpath(ctx), jr)
        emit_diag(ctx, operational_info("null-distill attested for $gid"))
        ctx.json && json_cli_out(Dict{String,Any}(
            "command" => "distill", "goal" => String(gid), "null" => true, "empty" => true,
        ))
        return EXIT_OK
    end
    pool = mass
    if pool == Set{String}([String(gid)])
        refs = goal_reference_sets(st)
        pool = Set{String}(id for (id, rs) in refs if String(gid) in rs)
    end
    cands = distill_candidates(st, pool)
    met = !isempty(linked) || attested
    if ctx.json
        json_cli_out(Dict{String,Any}(
            "command" => "distill",
            "goal" => String(gid),
            "precondition_met" => met,
            "linked_discoveries" => linked,
            "null_attested" => attested,
            "candidates" => [
                Dict{String,Any}("id" => id, "kind" => k, "title" => t, "skeleton" => distill_skeleton(id))
                for (id, k, t) in cands
            ],
        ))
        return EXIT_OK
    end
    println("distillation worksheet for ", gid, isempty(g.title) ? "" : " ($(g.title))")
    if met
        how = !isempty(linked) ? "linked Discovery: " * join(linked, ", ") : "null-distill attested"
        println("archive precondition: met (", how, ")")
    else
        println("archive precondition not met, archiving ", gid,
                " refuses until a discovery is linked or a null-distillation attestation exists")
    end
    if isempty(cands)
        println("no validated B / answered Q / accepted D in the goal's mass")
    else
        println("candidates:")
        for (id, k, t) in cands
            label = k == "b" ? "validated B" : k == "q" ? "answered Q" : "accepted D"
            println("- ", id, " (", label, "): ", t)
            println("    ", distill_skeleton(id))
        end
    end
    println("nothing worth distilling, attest a null distillation for ", gid)
    EXIT_OK
end

function cmd_renumber(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "renumber requires an id and target id"))
    haskey(kw, "to") || (return ferr(ctx, EXIT_ERR, "missing --to=<NEW-ID>"))
    old_id = strip(pos[1])
    new_id = strip(kw["to"])
    isempty(new_id) && (return ferr(ctx, EXIT_ERR, "bad --to"))
    old_id == new_id && return EXIT_OK
    st = load(ctx)
    ow = get(st.nodes, old_id, nothing)
    eff = effective_session_token(ctx.root, kw)
    ow !== nothing &&
        ow.kind === :w &&
        ow.status === :progress && begin
                msg = session_denial_progress_mutate(ow, eff)
                msg !== nothing && begin return fail_diag(ctx, EXIT_GUARD, msg) end
            end
    renumber_blocked_by_done_evidence(st, old_id) && begin
            fail_diag(ctx, EXIT_GUARD, invariant_error("renumber refuses, the id occurs in evidence on a done work item"))
            return EXIT_GUARD
        end
    try
        apply_renumber!(st, old_id, new_id)
    catch e
        fail_diag(ctx, EXIT_ERR, interface_error("renumber could not be applied",
            Pair{String,String}["error" => sprint(showerror, e)]))
        return EXIT_ERR
    end
    jr = wrap_journal_record("renumber",
                             Dict{String,Any}("op" => "renumber_swap", "from" => new_id,
                                               "to" => old_id))
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function cmd_resume(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "resume requires a work id"))
    id = pos[1]
    st = load(ctx)
    w = get(st.nodes, id, nothing)
    w === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    w.kind === :w || (return ferr(ctx, EXIT_ERR, "not a work item"))
    w.status === :progress || (return finv(ctx, EXIT_GUARD, "$(id) is not in progress"))
    eff = effective_session_token(ctx.root, kw)
    inv = merge(
        Dict{String,Any}("op" => "session_restore_claim", "id" => String(id)),
        session_journal_snap(w),
    )
    jr = wrap_journal_record("resume", inv)
    assign_w_claim_session!(w, eff)
    stamp_touch_node!(w)
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function cmd_handoff(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "handoff requires a work id and target session"))
    haskey(kw, "to") || (return ferr(ctx, EXIT_ERR, "missing --to=<session-token>"))
    to_tok = strip(String(kw["to"]))
    isempty(to_tok) && (return ferr(ctx, EXIT_ERR, "empty --to"))
    id = pos[1]
    st = load(ctx)
    w = get(st.nodes, id, nothing)
    w === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    w.kind === :w || (return ferr(ctx, EXIT_ERR, "not a work item"))
    w.status === :progress || (return finv(ctx, EXIT_GUARD, "$(id) is not in progress"))
    eff = effective_session_token(ctx.root, kw)
    !progress_has_session_record(w) && (return finv(ctx, EXIT_GUARD, "no session claim on $(id), adopt the claim before handing off"))
    !session_token_matches(w, eff) && (return finv(ctx, EXIT_GUARD, "only the holding session can hand off, adopt the claim first"))
    inv = merge(
        Dict{String,Any}("op" => "session_restore_claim", "id" => String(id)),
        session_journal_snap(w),
    )
    jr = wrap_journal_record("handoff", inv)
    assign_w_claim_session!(w, to_tok)
    stamp_touch_node!(w)
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function cmd_revert(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "revert requires a work id"))
    id = pos[1]
    st = load(ctx)
    w = get(st.nodes, id, nothing)
    w === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    w.kind === :w || (return ferr(ctx, EXIT_ERR, "not a work item"))
    w.status === :progress || (return finv(ctx, EXIT_GUARD, "$(id) is not in progress"))
    eff = effective_session_token(ctx.root, kw)
    msg = session_denial_progress_release(w, eff)
    msg !== nothing && (return fail_diag(ctx, EXIT_GUARD, msg))
    gs = Dict{String,String}()
    for gid in get(w.fields, :goals, String[])
        g = get(st.nodes, gid, nothing)
        g === nothing && continue
        gs[gid] = string(g.status)
    end
    inv = Dict{String,Any}(
        "op" => "set_w_status_with_goals",
        "id" => String(id),
        "old_w_status" => "progress",
        "goal_statuses" => gs,
    )
    merge!(inv, session_journal_snap(w))
    jr = wrap_journal_record("revert", inv)
    w.status = :ready
    clear_w_session_attrs!(w)
    rederive_goals!(st, w)
    stamp_touch_node!(w)
    persist(ctx, st; journal=jr, session=eff)
    EXIT_OK
end

function cmd_render(ctx::CliCtx, pos, kw)
    st = load(ctx)
    write_index(indexpath(ctx), st; decay=dashboard_decay_count(ctx, st))
    EXIT_OK
end

function cmd_undo(ctx::CliCtx, pos, kw)
    jp = journalpath(ctx)
    (!isfile(jp) || filesize(jp) == 0) && begin
            fail_diag(ctx, EXIT_ERR, system_error("no journal to undo", Pair{String,String}["path" => jp])); return EXIT_ERR
        end
    steps = if haskey(kw, "steps")
            v = tryparse(Int, kw["steps"])
            v === nothing && begin fail_diag(ctx, EXIT_ERR, interface_error("bad --steps, expected an integer")); return EXIT_ERR end
            max(Int(v), 0)
        else
            1
        end
    steps == 0 && return EXIT_OK
    view = journal_tail_mutation_view(jp, steps)
    view === nothing && begin
            fail_diag(ctx, EXIT_ERR, interface_error("journal has fewer than $(steps) mutation $(steps == 1 ? "entry" : "entries")", Pair{String,String}["steps" => string(steps)])); return EXIT_ERR
        end
    idxs, tail_recs = view
    st = load(ctx)
    for rec in reverse(tail_recs)
        inv = get(rec, "inv", nothing)
        inv isa AbstractDict || begin
            fail_diag(ctx, EXIT_INVARIANT, system_error("journal undo failed",
                Pair{String,String}["error" => "malformed record missing inv"]))
            return EXIT_INVARIANT
        end
        msg = journal_apply_inverse!(st, inv)
        msg !== nothing && begin return fail_diag(ctx, EXIT_INVARIANT, msg) end
        if get(inv, "op", "") == "glossary_rename_restore" && get(inv, "glossary_changed", false) === true &&
           haskey(inv, "old") && haskey(inv, "new")
            gpath = glossarypath(ctx)
            if isfile(gpath)
                reversed, gchanged = glossary_rename_in_text(read(gpath, String),
                    String(inv["new"]), String(inv["old"]))
                gchanged && write(gpath, reversed)
            end
        end
    end
    journal_reconcile_counters!(st)
    journal_drop_lines_inplace!(jp, idxs)
    persist(ctx, st)
    append_journal_record!(jp, wrap_journal_record("undo", Dict{String,Any}(
        "op" => "undo",
        "steps" => steps,
    ); session=journal_session_token(ctx, kw)))
    EXIT_OK
end

function cmd_repair(ctx::CliCtx, pos, kw)
    haskey(kw, "confirm") || (return ferr(ctx, EXIT_ERR, "refusing without --confirm"))
    p = lockpath(ctx)
    text = replace(read(p, String), "\r\n" => "\n")
    st, _, _ = parse_lock(text)
    persist(ctx, st)
    emit_diag(ctx, operational_info("recanonicalized the lock", Pair{String,String}["path" => p]))
    EXIT_OK
end

function cmd_ready(ctx::CliCtx, pos, kw)
    st = load(ctx)
    cp = Set(critical_path(st))
    rs = ready(st)
    sort!(rs; by=w -> ((w.id in cp) ? 0 : 1, -length(impact(st, w.id)), id_key(w.id)))
    if ctx.json
        items = Dict{String,Any}[
            Dict("id" => w.id, "title" => w.title, "critical" => w.id in cp) for w in rs
        ]
        json_cli_out(Dict("command" => "ready", "items" => items))
        return EXIT_OK
    end
    for w in rs
        flag = w.id in cp ? " [crit]" : ""
        println(w.id, "  ", w.title, flag)
    end
    EXIT_OK
end

function cmd_next(ctx::CliCtx, pos, kw)
    st = load(ctx)
    rs = ready(st)
    isempty(rs) && return fail_diag(ctx, EXIT_OK, operational_info("no ready work items"))
    cp = Set(critical_path(st))
    crit = filter(w -> w.id in cp, rs)
    pick = isempty(crit) ? first(rs) : first(crit)
    pkt = packet(st, pick)
    if ctx.json
        json_cli_out(Dict("command" => "next", "work" => pick.id, "packet_markdown" => pkt))
        return EXIT_OK
    end
    print(pkt)
    EXIT_OK
end

function cmd_packet(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "packet", "usage: grove packet <W-NN>"))
    depth = 4
    maxcount = 50
    if haskey(kw, "cone-depth")
        v = tryparse(Int, kw["cone-depth"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --cone-depth (expected integer)"))
        v < 1 && (return ferr(ctx, EXIT_ERR, "--cone-depth must be ≥ 1"))
        depth = v
    end
    if haskey(kw, "cone-max")
        v = tryparse(Int, kw["cone-max"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --cone-max (expected integer)"))
        v < 1 && (return ferr(ctx, EXIT_ERR, "--cone-max must be ≥ 1"))
        maxcount = v
    end
    st = load(ctx)
    n = get(st.nodes, pos[1], nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found"))
    n.kind === :w || (return ferr(ctx, EXIT_ERR, "not a work item"))
    cone = haskey(kw, "cone")
    pkt = packet(st, n)
    cone && (pkt = pkt * packet_cone(st, n; depth=depth, maxcount=maxcount))
    if ctx.json
        out = Dict{String,Any}("command" => "packet", "work" => n.id, "packet_markdown" => pkt)
        if cone
            back = backward_cone(st, n.id; depth=depth, maxcount=maxcount)
            fwd = forward_cone(st, n.id; depth=depth, maxcount=maxcount)
            out["cone"] = Dict{String,Any}(
                "backward" => back.ids,
                "order" => contraction_order(st, back.ids),
                "forward" => fwd.ids,
                "fragility" => [Dict{String,Any}("goal" => g, "paths" => k) for (g, k) in goal_fragility(st, n)],
                "relevant_discoveries" => relevant_discoveries(st, n, back.ids; maxcount=maxcount),
                "truncated" => back.truncated || fwd.truncated,
                "depth" => depth,
                "max" => maxcount,
            )
        end
        json_cli_out(out)
        return EXIT_OK
    end
    print(pkt)
    EXIT_OK
end

function cmd_deps(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "deps requires a node id"))
    st = load(ctx)
    pred = deps(st, pos[1])
    if ctx.json
        json_cli_out(Dict(
            "command" => "deps",
            "id" => String(pos[1]),
            "predecessors" => pred,
        ))
        return EXIT_OK
    end
    for id in pred
        println(id)
    end
    EXIT_OK
end

function cmd_impact(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "impact requires a node id"))
    st = load(ctx)
    succ = impact(st, pos[1])
    if ctx.json
        json_cli_out(Dict(
            "command" => "impact",
            "id" => String(pos[1]),
            "successors" => succ,
        ))
        return EXIT_OK
    end
    for id in succ
        println(id)
    end
    EXIT_OK
end

function cmd_path(ctx::CliCtx, pos, kw)
    st = load(ctx)
    chain = critical_path(st)
    if ctx.json
        json_cli_out(Dict("command" => "path", "chain" => chain))
        return EXIT_OK
    end
    for id in chain
        println(id)
    end
    EXIT_OK
end

function cmd_triage(ctx::CliCtx, pos, kw)
    st = load(ctx)
    rows = triage_rows(st)
    if ctx.json
        json_cli_out(Dict{String,Any}(
            "command" => "triage",
            "rows" => [
                Dict{String,Any}(
                    "w" => r.w,
                    "title" => r.title,
                    "coverage" => r.cov,
                    "declared" => r.declared,
                    "uncertainty" => r.uncertainty,
                    "fragile" => r.fragile,
                    "suggestion" => r.suggestion,
                ) for r in rows
            ],
        ))
        return EXIT_OK
    end
    isempty(rows) && (println("triage: no open work"); return EXIT_OK)
    println("W\tcov\tχ\tfragile\tsuggestion")
    for r in rows
        println(r.w, "\t", @sprintf("%.2f", r.cov), "\t", r.uncertainty, "\t",
            r.fragile ? "yes" : "no", "\t", r.suggestion)
    end
    EXIT_OK
end

function cmd_dor(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "dor requires a work id"))
    st = load(ctx)
    n = get(st.nodes, pos[1], nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $(pos[1])"))
    if ctx.json
        conj = [
            Dict{String,Any}("label" => label, "ok" => ok, "detail" => detail)
            for (label, ok, detail) in dor_breakdown(st, n)
        ]
        json_cli_out(Dict(
            "command" => "dor",
            "work" => n.id,
            "conjuncts" => conj,
            "dor" => dor(st, n),
        ))
        return EXIT_OK
    end
    println(n.id, " DoR:")
    for (label, ok, detail) in dor_breakdown(st, n)
        sym = ok ? "⊤" : "⊥"
        if isempty(detail)
            println("  ", sym, "  ", label)
        else
            println("  ", sym, "  ", label, "  → ", detail)
        end
    end
    overall = dor(st, n) ? "⊤" : "⊥"
    println("result: ", overall)
    EXIT_OK
end

function cmd_show(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return ferr(ctx, EXIT_ERR, "show requires a node id"))
    st = load(ctx)
    n = get(st.nodes, pos[1], nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $(pos[1])"))
    if ctx.json
        json_cli_out(json_node_snapshot(n))
        return EXIT_OK
    end
    io = IOBuffer()
    serialize_node!(io, n)
    print(String(take!(io)))
    EXIT_OK
end

function cmd_list(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "list", "usage: grove list <kind>"))
    kind = Symbol(pos[1])
    st = load(ctx)
    rows = listnodes(st, kind)
    fstatus = get(kw, "status", "")
    fcynefin = get(kw, "cynefin", "")
    outrows = Dict{String,Any}[]
    for n in rows
        isempty(fstatus) || String(n.status) == fstatus || continue
        isempty(fcynefin) || (n.cynefin !== nothing && String(n.cynefin) == fcynefin) || continue
        row = Dict{String,Any}(
            "id" => n.id,
            "status" => string(n.status),
            "title" => n.title,
        )
        if n.cynefin !== nothing
            row["cynefin"] = string(n.cynefin)
        end
        push!(outrows, row)
    end
    if ctx.json
        d = Dict{String,Any}(
            "command" => "list",
            "kind" => String(kind),
            "rows" => outrows,
        )
        isempty(fstatus) || (d["filter_status"] = fstatus)
        isempty(fcynefin) || (d["filter_cynefin"] = fcynefin)
        json_cli_out(d)
        return EXIT_OK
    end
    for n in rows
        isempty(fstatus) || String(n.status) == fstatus || continue
        isempty(fcynefin) || (n.cynefin !== nothing && String(n.cynefin) == fcynefin) || continue
        println(n.id, "\t", n.status, "\t", n.title)
    end
    EXIT_OK
end

function cmd_graph(ctx::CliCtx, pos, kw)
    st = load(ctx)
    io = IOBuffer()
    render_graph!(io, st)
    text = String(take!(io))
    if ctx.json
        json_cli_out(Dict("command" => "graph", "mermaid" => text))
        return EXIT_OK
    end
    print(text)
    EXIT_OK
end

function cmd_log(ctx::CliCtx, pos, kw)
    st = load(ctx)
    lfilt = nothing
    if length(pos) >= 1
        id0 = pos[1]
        jp0 = journalpath(ctx)
        ok = haskey(st.nodes, id0) || any(e -> e.from == id0 || e.to == id0, st.edges)
        !ok && journal_file_mentions_id(jp0, id0) && (ok = true)
        !ok && (return ferr(ctx, EXIT_NOTFOUND, "not found $id0"))
        lfilt = id0
    end
    lim = if haskey(kw, "limit")
        v = tryparse(Int, kw["limit"])
        v === nothing && begin
                (return ferr(ctx, EXIT_ERR, "bad --limit (expected integer)"))
            end
        v::Int
    else
        200
    end
    rows = log_timeline(st; idfilt=lfilt, limit=lim, journal_path=journalpath(ctx))
    if ctx.json
        jr = [
            Dict{String,Any}("ts" => r.ts, "sort" => r.tiebreaker, "line" => r.line) for r in rows
        ]
        d = Dict{String,Any}(
            "command" => "log",
            "limit" => lim,
            "rows" => jr,
        )
        lfilt === nothing || (d["id_filter"] = lfilt)
        json_cli_out(d)
        return EXIT_OK
    end
    print_timeline(rows)
    EXIT_OK
end

function cmd_diff(ctx::CliCtx, pos, kw)
    ref = get(kw, "since", "HEAD")
    rp = abspath(ctx.root)
    git_repository_root(rp) || begin
            fail_diag(ctx, EXIT_ERR, system_error(
                "not a git repository, cannot resolve the lock through git",
                Pair{String,String}["root" => rp]))
            return EXIT_ERR
        end
    wt_path = lockpath(ctx)
    isfile(wt_path) || begin
            fail_diag(ctx, EXIT_ERR, system_error(
                "lock file is absent, initialize the project first",
                Pair{String,String}["path" => wt_path]))
            return EXIT_ERR
        end
    wt_text = read_worktree_lock_text(wt_path)
    st_wt = try
        parse_lock(wt_text)[1]
    catch e
        e isa LockParseError || rethrow()
        fail_diag(ctx, EXIT_ERR, system_error("worktree lock could not be parsed",
            Pair{String,String}["error" => sprint(showerror, e)]))
        return EXIT_ERR
    end
    blob, gerr = git_show_path(rp, ref, ".grove/state.lock")
    blob === nothing && begin
            fail_diag(ctx, EXIT_ERR, system_error("lock could not be resolved from git",
                Pair{String,String}["error" => String(gerr)]))
            return EXIT_ERR
        end
    st_ref = try
        parse_lock(blob)[1]
    catch e
        e isa LockParseError || rethrow()
        fail_diag(ctx, EXIT_ERR, system_error("reference lock could not be parsed",
            Pair{String,String}["error" => sprint(showerror, e), "git_ref" => ref]))
        return EXIT_ERR
    end
    if ctx.json
        pl = lock_structural_diff_payload(st_ref, st_wt)
        pl["command"] = "diff"
        pl["since"] = ref
        json_cli_out(pl)
        return EXIT_OK
    end
    print_lock_structural_diff(stdout, ref, st_ref, st_wt)
    EXIT_OK
end

function cmd_status(ctx::CliCtx, pos, kw)
    st = load(ctx)
    eff = effective_session_token(ctx.root, kw)
    prog = Node[w for w in listnodes(st, :w) if w.status === :progress]
    sort!(prog; by=w -> id_key(w.id))
    if ctx.json
        items = Dict{String,Any}[]
        for w in prog
            tok = progress_has_session_record(w) ? String(w.attrs["session"]) : ""
            stale = progress_session_display_stale(w, eff)
            line2 = if isempty(tok)
                        "  (no session on record, I11 broken, adopt the claim or re-claim progress)"
                    else
                        flag = session_token_matches(w, eff) ? "" : "  [!= this session]"
                        age = session_claim_age_stale(w) ? "  (claimed >$(SESSION_DISPLAY_STALE_AFTER_HOURS)h ago)" : ""
                        string("  session=", tok, flag, age)
                    end
            opts = stale ? "resume, revert, or handoff the claim" : ""
            push!(items, Dict{String,Any}(
                "id" => w.id,
                "title" => w.title,
                "session" => tok,
                "stale_for_agent" => stale,
                "session_detail" => line2,
                "options_hint" => opts,
            ))
        end
        al = alignment_triggers(st)
        inv = check_all(st)
        json_cli_out(Dict(
            "command" => "status",
            "progress" => items,
            "alignment_triggers" => al,
            "invariants" => Dict(
                "ok" => isempty(inv),
                "messages" => Any[diag_json_obj(e) for e in inv],
            ),
        ))
        return EXIT_OK
    end
    println("# Status")
    println()
    println("## Work in `progress`")
    println()
    if isempty(prog)
        println("(none)")
    else
        for w in prog
            tok = progress_has_session_record(w) ? String(w.attrs["session"]) : ""
            stale = progress_session_display_stale(w, eff)
            line2 = if isempty(tok)
                        "  (no session on record, I11 broken, adopt the claim or re-claim progress)"
                    else
                        flag = session_token_matches(w, eff) ? "" : "  [!= this session]"
                        age = session_claim_age_stale(w) ? "  (claimed >$(SESSION_DISPLAY_STALE_AFTER_HOURS)h ago)" : ""
                        string("  session=", tok, flag, age)
                    end
            if stale
                println(w.id, "\t", w.title, "  (stale for this agent)\n", line2)
                println("  options: resume, revert, or handoff the claim for $(w.id)")
            else
                println(w.id, "\t", w.title, "\n", line2)
            end
        end
    end
    println()
    println("## Alignment triggers")
    println()
    al = alignment_triggers(st)
    if isempty(al)
        println("(none)")
    else
        for line in al
            println("- ", line)
        end
    end
    println()
    println("## Invariants")
    println()
    inv = check_all(st)
    if isempty(inv)
        println("ok")
    else
        for e in inv
            println("- ", e)
        end
    end
    EXIT_OK
end

function cmd_gate(ctx::CliCtx, pos, kw)
    theta = 0
    if haskey(kw, "theta")
        v = tryparse(Int, kw["theta"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --theta (expected integer)"))
        v < 0 && (return ferr(ctx, EXIT_ERR, "--theta must be ≥ 0"))
        theta = Int(v)
    end
    n = 5
    if haskey(kw, "n")
        v = tryparse(Int, kw["n"])
        v === nothing && (return ferr(ctx, EXIT_ERR, "bad --n (expected integer)"))
        v < 1 && (return ferr(ctx, EXIT_ERR, "--n must be ≥ 1"))
        n = Int(v)
    end
    st = load(ctx)
    jp = journalpath(ctx)
    _, recs = journal_read_nonempty_pairs(jp)
    rep = gate_report(st, recs, ctx.root; theta=theta, n=n)
    jr = wrap_journal_record("gate", Dict{String,Any}(
        "op" => JOURNAL_GATE_OP,
        "tw" => rep.tw_now,
        "dones" => rep.dones,
        "empty" => rep.empty,
        "overflows" => String[String(wid) for (wid, _) in rep.overflows],
        "overflow_counts" => Dict{String,Any}(String(wid) => length(paths) for (wid, paths) in rep.overflows),
        "invalidated" => String[String(b.id) for b in rep.invalidated],
    ); session=journal_session_token(ctx, kw))
    append_journal_record!(jp, jr)
    if ctx.json
        json_cli_out(gate_json_payload(rep))
        return EXIT_OK
    end
    println("baseline: ", rep.baseline === nothing ? "none" : rep.baseline.ts)
    println("treewidth: ", rep.tw_now, " (Δ ", rep.tw_delta >= 0 ? "+" : "", rep.tw_delta, ")")
    println("done since baseline: ", rep.dones)
    println("due: ", rep.due)
    if isempty(rep.overflows) && isempty(rep.invalidated) && isempty(rep.accepted)
        println("would distill: none")
        return EXIT_OK
    end
    println("would distill:")
    for (wid, paths) in rep.overflows
        println("- overflow ", wid, ": ", join(paths, ", "))
    end
    for b in rep.invalidated
        println("- invalidated ", b.id, ": ", b.title)
    end
    for d in rep.accepted
        println("- accepted ", d.id, ": ", d.title)
    end
    EXIT_OK
end

function cmd_stats(ctx::CliCtx, pos, kw)
    st = load(ctx)
    _, recs = journal_read_nonempty_pairs(journalpath(ctx))
    payload = compute_stats(st, recs)
    if ctx.json
        json_cli_out(payload)
        return EXIT_OK
    end
    print_stats_human(payload)
    EXIT_OK
end

function cmd_revalidate(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "revalidate", "usage: grove revalidate <Y-NN> [--surface=p1,p2] [--from=ID,...]"))
    id = pos[1]
    st = load(ctx)
    n = get(st.nodes, id, nothing)
    n === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    n.kind === :y || (return ferr(ctx, EXIT_ERR, "$id is not a discovery"))
    n.status === :stale || (return finv(ctx, EXIT_GUARD, "$id is $(n.status), not stale"))
    has_surface = haskey(kw, "surface")
    has_from = haskey(kw, "from")
    (has_surface || has_from) ||
        (return finv(ctx, EXIT_GUARD, "refusing without payment, pass --surface=<paths> and/or --from=<ID>"))
    surface = String[String(s) for s in split(get(kw, "surface", ""), ',') if !isempty(strip(s))]
    froms = String[String(t) for t in split(get(kw, "from", ""), ',') if !isempty(strip(t))]
    has_surface && isempty(surface) && (return ferr(ctx, EXIT_ERR, "--surface given but empty"))
    has_from && isempty(froms) && (return ferr(ctx, EXIT_ERR, "--from given but empty"))
    for p in surface
        pth = isabspath(p) ? p : joinpath(ctx.root, p)
        ispath(pth) || (return fail_diag(ctx, EXIT_GUARD, system_error("surface path does not exist under root", Pair{String,String}["path" => p])))
    end
    for oid in froms
        src = get(st.nodes, oid, nothing)
        src === nothing && (return ferr(ctx, EXIT_GUARD, "unknown --from id $oid"))
        src.kind in (:w, :d, :q, :b) ||
            (return ferr(ctx, EXIT_GUARD, "--from $oid must reference W or D, Q or B"))
        (src.kind === :d && src.status === :superseded) &&
            (return finv(ctx, EXIT_GUARD, "--from $oid is superseded"))
        (src.kind === :b && src.status in (:invalidated_acceptable, :invalidated_blocking)) &&
            (return finv(ctx, EXIT_GUARD, "--from $oid is invalidated"))
    end
    paid = String[]
    has_surface && push!(paid, "surface=" * join(surface, ","))
    has_from && push!(paid, "from=" * join(froms, ","))
    line = utc_stamp_second() * " " * join(paid, " ")
    added = Dict{String,Any}[]
    for oid in froms
        src = st.nodes[oid]
        f0, l0, t0 = src.kind === :w ? (oid, :produces, id) : (id, :distills, oid)
        already = any(e -> e.from == f0 && e.label === l0 && e.to == t0, st.edges)
        r = validate_and_push_edge!(st, f0, l0, t0)
        r !== nothing && return fail_diag(ctx, EXIT_GUARD, r)
        already || push!(added, Dict{String,Any}("from" => f0, "label" => String(l0), "to" => t0))
    end
    jr = wrap_journal_record("revalidate", Dict{String,Any}(
        "op" => "revalidate_restore",
        "id" => String(id),
        "old_status" => string(n.status),
        "had_surface" => haskey(n.fields, :surface),
        "old_surface" => String[String(s) for s in get(n.fields, :surface, String[])],
        "added_edges" => added,
    ))
    n.status = :active
    has_surface && (n.fields[:surface] = surface)
    push!(get!(n.fields, :revalidation, String[]), line)
    stamp_touch_node!(n)
    persist(ctx, st; journal=jr, session=journal_session_token(ctx, kw))
    ctx.json && json_cli_out(Dict{String,Any}(
        "command" => "revalidate", "id" => String(id), "status" => "active", "line" => line,
    ))
    EXIT_OK
end

function cmd_glossary(ctx::CliCtx, pos, kw)
    (length(pos) >= 1 && pos[1] == "rename") ||
        (return ferr(ctx, EXIT_ERR, "glossary requires the rename operation"))
    cmd_glossary_rename(ctx, pos[2:end], kw)
end

function cmd_glossary_rename(ctx::CliCtx, pos, kw)
    length(pos) >= 2 || (return ferr(ctx, EXIT_ERR, "glossary rename requires old and new terms"))
    old = String(strip(pos[1]))
    new = String(strip(pos[2]))
    (isempty(old) || isempty(new)) && (return ferr(ctx, EXIT_ERR, "glossary rename requires nonempty terms"))
    old == new && (return ferr(ctx, EXIT_ERR, "glossary rename terms are identical"))
    st = load(ctx)
    gpath = glossarypath(ctx)
    terms = glossary_terms(gpath)
    users = Node[x for x in listnodes(st, :y) if any(t -> String(t) == old, get(x.fields, :tags, String[]))]
    in_glossary = old in terms
    if !in_glossary && isempty(users)
        (return ferr(ctx, EXIT_NOTFOUND, "glossary term $old is absent and unused by discoveries"))
    end
    new in terms &&
        (return finv(ctx, EXIT_GUARD, "glossary term $new already exists"))
    changed_in_glossary = false
    if in_glossary
        renamed, changed = glossary_rename_in_text(read(gpath, String), old, new)
        changed || (return ferr(ctx, EXIT_NOTFOUND, "glossary term $old is absent"))
        write(gpath, renamed)
        changed_in_glossary = true
    end
    snap = Dict{String,Any}()
    for x in users
        tags = String[String(t) for t in get(x.fields, :tags, String[])]
        snap[x.id] = tags
        x.fields[:tags] = unique(String[t == old ? new : t for t in tags])
        stamp_touch_node!(x)
    end
    jr = wrap_journal_record("glossary", Dict{String,Any}(
        "op" => "glossary_rename_restore",
        "tags" => snap,
        "old" => old,
        "new" => new,
        "glossary_changed" => changed_in_glossary,
    ))
    persist(ctx, st; journal=jr, session=journal_session_token(ctx, kw))
    ctx.json && json_cli_out(Dict{String,Any}(
        "command" => "glossary", "subcommand" => "rename",
        "old" => old, "new" => new, "nodes" => String[x.id for x in users],
    ))
    EXIT_OK
end

function glossary_terms(gpath::AbstractString)::Set{String}
    terms = Set{String}()
    isfile(gpath) || return terms
    for line in eachline(gpath)
        s = strip(line)
        startswith(s, "|") || continue
        cells = split(s, "|")
        length(cells) >= 3 || continue
        term = strip(String(cells[2]))
        (isempty(term) || term == "Term" || all(c -> c == '-', term)) && continue
        push!(terms, term)
    end
    terms
end

function glossary_rename_in_text(text::AbstractString, old::String, new::String)::Tuple{String,Bool}
    changed = false
    lines = String[String(l) for l in split(text, '\n')]
    for (i, line) in pairs(lines)
        s = strip(line)
        startswith(s, "|") || continue
        cells = split(line, "|")
        length(cells) >= 3 || continue
        cell = String(cells[2])
        stripped_cell = strip(cell)
        (isempty(stripped_cell) || stripped_cell == "Term" || all(c -> c == '-', stripped_cell)) && continue
        stripped_cell == old || continue
        idx = findfirst(old, cell)
        idx === nothing && continue
        cells[2] = cell[1:prevind(cell, first(idx))] * new * cell[nextind(cell, last(idx)):end]
        lines[i] = join(cells, "|")
        changed = true
    end
    (join(lines, "\n"), changed)
end

function discovery_decay_errors(st::State, root::AbstractString, gpath::AbstractString)::Vector{Diagnostic}
    out = Diagnostic[]
    xs = Node[x for x in listnodes(st, :y) if x.status in (:proposed, :active)]
    isempty(xs) && return out
    terms = glossary_terms(gpath)
    for x in xs
        if haskey(x.fields, :surface)
            for p in get(x.fields, :surface, String[])
                pth = isabspath(String(p)) ? String(p) : joinpath(root, String(p))
                ispath(pth) || push!(out, invariant_error("$(x.id) has a dead surface path",
                    Pair{String,String}["path" => String(p)]))
            end
        end
        for e in st.edges
            (e.label === :distills && e.from == x.id) || continue
            dst = get(st.nodes, e.to, nothing)
            dst === nothing && continue
            dst.archived && continue
            if dst.kind === :d && dst.status === :superseded
                push!(out, invariant_error("$(x.id) has a rotted origin ($(dst.id))",
                    Pair{String,String}["origin" => String(dst.id), "status" => String(dst.status)]))
            elseif dst.kind === :b && dst.status in (:invalidated_acceptable, :invalidated_blocking)
                push!(out, invariant_error("$(x.id) has a rotted origin ($(dst.id))",
                    Pair{String,String}["origin" => String(dst.id), "status" => String(dst.status)]))
            end
        end
        for t in get(x.fields, :tags, String[])
            String(t) in terms || push!(out, invariant_error("$(x.id) lost a glossary term ($(String(t)))",
                Pair{String,String}["term" => String(t)]))
        end
    end
    out
end

function cmd_check(ctx::CliCtx, pos, kw)
    st = try
        load(ctx; verify=true)
    catch e
        rethrow()
    end
    errs = check_all(st)
    append!(errs, discovery_decay_errors(st, ctx.root, glossarypath(ctx)))
    if ctx.json
        json_cli_out(Dict(
            "command" => "check",
            "ok" => isempty(errs),
            "errors" => Any[diag_json_obj(e) for e in errs],
        ))
        return isempty(errs) ? EXIT_OK : EXIT_INVARIANT
    end
    if isempty(errs)
        emit_diag(ctx, operational_info("ok"))
        return EXIT_OK
    end
    for e in errs; println(stderr, render_diag(e)); end
    EXIT_INVARIANT
end

function cmd_projects(ctx::CliCtx, pos, kw)
    reg = registry_load()
    if reg === nothing
        emit_diag(ctx, system_warning("malformed registry, registry features are disabled",
            Pair{String,String}["path" => registry_path()]))
        reg = ProjectEntry[]
    end
    if ctx.json
        json_cli_out(Dict{String,Any}(
            "command" => "projects",
            "projects" => [Dict{String,Any}(
                "name" => e.name, "path" => e.path,
                "created" => e.created, "last_opened" => e.last_opened) for e in reg],
        ))
        return EXIT_OK
    end
    for e in reg
        println(e.name, "\t", e.path, "\t", e.last_opened)
    end
    EXIT_OK
end

function cmd_promote(ctx::CliCtx, pos, kw)
    length(pos) >= 1 || (return fusage(ctx, "promote", "usage: grove promote Y-NN --to=<project>"))
    id = String(pos[1])
    to = get(kw, "to", nothing)
    (to === nothing || isempty(strip(to))) &&
        (return ferr(ctx, EXIT_ERR, "missing --to=<project>"))
    st = load(ctx)
    src = get(st.nodes, id, nothing)
    src === nothing && (return ferr(ctx, EXIT_NOTFOUND, "not found $id"))
    src.kind === :y ||
        (return ferr(ctx, EXIT_ERR, "$id is kind $(src.kind), not y"))
    target_root = resolve_project_target(to)
    target_root === nothing && return EXIT_NOTFOUND
    abspath(target_root) == abspath(ctx.root) &&
        (return ferr(ctx, EXIT_ERR, "target is the source project"))
    tctx = CliCtx(target_root, ctx.quiet, ctx.json, ctx.no_render)
    isfile(lockpath(tctx)) ||
        (return fail_diag(ctx, EXIT_ERR, system_error("target lock is absent, initialize the target project first",
            Pair{String,String}["path" => lockpath(tctx), "root" => target_root])))
    with_session_exclusive(tctx, () -> promote_into_target(ctx, src, tctx))
end

function promote_into_target(sctx::CliCtx, src::Node, tctx::CliCtx)::Int
    tst = load(tctx)
    reg = registry_load()
    reg === nothing && (reg = ProjectEntry[])
    origin_project = something(registry_name_for_path(reg, sctx.root),
                               basename(normpath(abspath(sctx.root))))
    origin_id = src.id
    for x in listnodes(tst, :y)
        if get(x.attrs, "origin_project", "") == origin_project &&
           get(x.attrs, "origin_id", "") == origin_id
            println(stderr, render_diag(invariant_error("already promoted as $(x.id)")))
            return EXIT_GUARD
        end
    end
    nid = next_id!(tst, :y)
    n = Node(:y, nid)
    n.title = src.title
    n.status = :proposed
    for f in (:tags, :surface, :invariant, :why, :skill_updates, :glossary_updates)
        haskey(src.fields, f) && (n.fields[f] = deepcopy(src.fields[f]))
    end
    n.attrs["origin_project"] = origin_project
    n.attrs["origin_id"] = origin_id
    n.attrs["origin_version"] = get(src.attrs, "t_updated", "")
    stamp_new_node!(n)
    tst.nodes[nid] = n
    promote_glossary_terms!(glossarypath(tctx),
                            String[String(t) for t in get(n.fields, :tags, String[])],
                            origin_project)
    persist(tctx, tst; journal=wrap_journal_record("promote", journal_inverse_rm_node(nid)))
    tctx.json && json_cli_out(Dict{String,Any}(
        "command" => "promote", "id" => nid,
        "origin_project" => origin_project, "origin_id" => origin_id,
    ))
    EXIT_OK
end

function promote_glossary_terms!(gpath::AbstractString, tags::Vector{String},
                                 origin_project::AbstractString)::Nothing
    isempty(tags) && return nothing
    terms = glossary_terms(gpath)
    missing = String[t for t in tags if !(t in terms)]
    isempty(missing) && return nothing
    if !isfile(gpath)
        mkpath(dirname(gpath))
        open(gpath, "w") do io
            println(io, "| Term | Meaning |")
            println(io, "| --- | --- |")
        end
    end
    open(gpath, "a") do io
        for t in missing
            println(io, "| ", t, " | copied from ", origin_project, " |")
        end
    end
    nothing
end

const COMMANDS = Dict{String,Function}(
    "init" => cmd_init,
    "add" => cmd_add,
    "set" => cmd_set,
    "field" => cmd_field,
    "link" => cmd_link,
    "unlink" => cmd_unlink,
    "evidence" => cmd_evidence,
    "fitness" => cmd_fitness,
    "archive" => cmd_archive,
    "distill" => cmd_distill,
    "render" => cmd_render,
    "repair" => cmd_repair,
    "ready" => cmd_ready,
    "next" => cmd_next,
    "packet" => cmd_packet,
    "deps" => cmd_deps,
    "impact" => cmd_impact,
    "path" => cmd_path,
    "triage" => cmd_triage,
    "dor" => cmd_dor,
    "show" => cmd_show,
    "list" => cmd_list,
    "graph" => cmd_graph,
    "check" => cmd_check,
    "status" => cmd_status,
    "stats" => cmd_stats,
    "diff" => cmd_diff,
    "log" => cmd_log,
    "renumber" => cmd_renumber,
    "resume" => cmd_resume,
    "handoff" => cmd_handoff,
    "revert" => cmd_revert,
    "undo" => cmd_undo,
    "gate" => cmd_gate,
    "revalidate" => cmd_revalidate,
    "glossary" => cmd_glossary,
    "skill" => cmd_skill,
    "specification" => cmd_specification,
    "projects" => cmd_projects,
    "promote" => cmd_promote,
)

const HELP = """
grove (graph-driven reasoning over verified evidence)

Read:
  ready                        list work items ready to start (critical first)
  next                         propose single next W with full execution packet
  packet    <W-NN>             full execution packet for a W [--cone --cone-depth=N --cone-max=N]
  deps      <ID>               transitive blocks-predecessors
  impact    <ID>               transitive blocks-successors
  path                         critical path (longest unfinished blocks chain)
  triage                       rank open W by discovery need (cov, χ, fragility; read-only advisory)
  dor       <W-NN>             DoR conjunct breakdown
  show      <ID>               record dump
  list      <kind>             list nodes (g|w|d|q|b|t|y|a) [--status= --cynefin=]
  check                        verify lock checksum and invariants
  graph                        print mermaid block
  status                       summary: progress work, alignment triggers, invariant notes
  stats                        read-only telemetry from journal + lock (cycle time, DoR, bets, discovery, undo, surprise, C/V)
  diff                         structural diff vs git ref (--since=REF, default HEAD)
  projects                     registry table: name, path, last opened
  skill     [--install=<dir>]  print the agent skill or install it with specification.md
  specification                print the embedded normative specification
  log       [<ID>]             timeline from t_* on nodes/edges + journal.log (--limit=N, default 200; 0=unlimited)
  gate                         report-only distillation gate: tw delta, surface overflows, invalidated B, accepted D [--theta=N] [--n=N]

Mutate:
  init                                       create .grove/state.lock + index.md + glossary.md [--id-stride=N] [--id-offset=K] [--id-width=W]
  add         <kind> --title="…" [...]       create node; prints assigned ID
  set         <ID> <key>=<value>             guarded transitions
  field       <ID> <field> add|rm|clear "…"  append one entry, remove the nth, or clear all
  link        <from> <label> <to>            labels: blocks|implements|asks|tests|targets|produces|causes|supersedes|distills
  unlink      <from> <label> <to>            remove an edge
  evidence    <W-NN> "…"                     append evidence line
  fitness     <W-NN> <G-NN> <±N>             set per-goal delta
  archive     <G-NN>                         archive G + exclusive w/d/q/b/t (requires distillation: a linked Discovery or `grove distill G-NN --null`)
  distill     <G-NN> [--null]                distillation worksheet for a verified goal; --null writes a null-distill attestation (journal, non-mutation)
  renumber    <ID> --to=<NEW-ID>             rewrite record + refs (not if id in done evidence)
  undo        [--steps=N]                    revert last N mutations (truncates `.grove/journal.log`)
  resume      <W-NN>                         adopt session token on a `progress` W (journal undo restores prior claim)
  handoff     <W-NN> --to=<token>            transfer ownership (holder only)
  revert      <W-NN>                         `progress` -> `ready`, clear session (holder or stale claim)
  revalidate  <Y-NN> --surface=…|--from=ID   `stale` Discovery -> `active`, paid with a fresh anchor
  promote     <Y-NN> --to=<project>          copy a Discovery into another project with origin provenance; copy starts `proposed`
  glossary    rename <old> <new>             rewrite glossary.md term + Discovery tags atomically
  render                                     regenerate index.md
  repair      --confirm                      accept current lock contents (recompute checksum)

Global flags: --root=<path> --project=<name|path> --quiet --json --no-render [--session=<token>]  (--since for diff; --limit for log; --steps for undo)
Root resolution: --root wins; else --project / GROVE_PROJECT (directory or registry name); else walk up from cwd to the first dir containing .grove/state.lock.
"""

const SESSION_READ_COMMANDS = Set([
    "ready", "next", "packet", "deps", "impact", "path", "dor", "triage",
    "show", "list", "graph", "check", "status", "diff", "log", "stats",
    "projects", "promote", "skill", "specification",
])

const SESSION_MUTATE_COMMANDS = Set([
    "init", "add", "set", "field", "link", "unlink", "evidence", "fitness",
    "archive", "distill", "repair", "render", "undo", "renumber",
    "resume", "handoff", "revert", "gate", "revalidate", "glossary",
])

include("session_lock.jl")

function main(args::Vector{String})::Int
    isempty(args) && (print(HELP); return EXIT_OK)
    if args[1] in ("-h", "--help", "help")
        print(HELP); return EXIT_OK
    end
    cmd = args[1]
    rest = args[2:end]
    ctx, pos, kw = parse_args(rest)
    fn = get(COMMANDS, cmd, nothing)
    fn === nothing && (ferr(ctx, EXIT_ERR, "unknown command $cmd"); print(stderr, HELP); return EXIT_ERR)
    root_given = any(a -> a == "--root" || startswith(a, "--root="), rest)
    resolved = resolve_root(ctx.root, kw, root_given)
    resolved === nothing && return EXIT_NOTFOUND
    ctx.root = resolved
    registry_note_open(ctx.root, cmd)
    thunk = () -> fn(ctx, pos, kw)
    try
        rc = if cmd in SESSION_READ_COMMANDS
            with_session_shared(ctx, thunk)
        elseif cmd in SESSION_MUTATE_COMMANDS
            with_session_exclusive(ctx, thunk)
        else
            thunk()
        end
        return rc
    catch e
        if e isa CliFailure
            return fail_diag(ctx, e.code, e.diag)
        end
        if e isa SessionLockTimeoutError
            emit_diag(ctx, e.diag)
            return EXIT_GUARD
        end
        if e isa LockParseError
            fail_diag(ctx, EXIT_ERR, system_error("lock file could not be parsed",
                Pair{String,String}["error" => sprint(showerror, e)]))
            return EXIT_ERR
        end
        rethrow()
    end
end

main(args::AbstractVector) = main(String.(collect(args)))
