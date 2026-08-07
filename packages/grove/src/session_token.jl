"""Session tokens on `progress` turn stale after this wall-clock age (RFC3339 UTC on `session_at`)."""
const SESSION_DISPLAY_STALE_AFTER_HOURS = 24

progress_has_session_record(w::Node)::Bool =
    haskey(w.attrs, "session") && !isempty(strip(String(w.attrs["session"])))

function effective_session_token(root::AbstractString, kw::AbstractDict)::String
    if haskey(kw, "session")
        t = strip(String(kw["session"]))
        isempty(t) || return t
    end
    env = strip(get(ENV, "GROVE_SESSION", ""))
    isempty(env) || return env
    derive_default_session_token(root)
end

"""Deterministic fallback: `hostname:hex16(norm_root)`."""
function host_slug_for_session()::String
    for k in ("COMPUTERNAME", "HOSTNAME", "HOST")
        v = strip(get(ENV, k, ""))
        isempty(v) || return lowercase(v)
    end
    "host"
end

function derive_default_session_token(root::AbstractString)::String
    rp = lowercase(replace(abspath(String(root)), '\\' => '/'))
    dig = bytes2hex(SHA.sha256(codeunits(String(rp))))[1:16]
    string(host_slug_for_session(), ":", dig)
end

function assign_w_claim_session!(w::Node, token::AbstractString)::Nothing
    w.attrs["session"] = strip(String(token))
    w.attrs["session_at"] = utc_stamp_second()
    nothing
end

function clear_w_session_attrs!(w::Node)::Nothing
    delete!(w.attrs, "session")
    delete!(w.attrs, "session_at")
    nothing
end

function session_token_matches(w::Node, eff::AbstractString)::Bool
    progress_has_session_record(w) || return false
    strip(String(w.attrs["session"])) == strip(String(eff))
end

function parse_rfc3339_utc_second(s)::Union{Nothing,Dates.DateTime}
    s = strip(String(s))
    length(s) < 20 && return nothing
    endswith(s, 'Z') || return nothing
    Dates.tryparse(DateTime, s[1:prevind(s, end, 1)], dateformat"yyyy-mm-ddTHH:MM:SS")
end

function session_claim_age_stale(w::Node)::Bool
    sa = get(w.attrs, "session_at", "")
    ts = parse_rfc3339_utc_second(sa)
    ts === nothing && return false
    Dates.now(Dates.UTC) - ts > Dates.Hour(SESSION_DISPLAY_STALE_AFTER_HOURS)
end

"""True when `grove status` should flag this row (mismatching agent or old claim)."""
function progress_session_display_stale(w::Node, eff::AbstractString)::Bool
    w.status !== :progress && return false
    !progress_has_session_record(w) && return true
    !session_token_matches(w, eff) && return true
    session_claim_age_stale(w)
end

function session_release_denied_message(w::Node)::String
    "I11/session: cannot release $(w.id): token differs and claim is fresh (<$(SESSION_DISPLAY_STALE_AFTER_HOURS)h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait"
end

function session_mutate_denied_message(w::Node)::String
    "I11/session: $(w.id) is `progress` and owned by another session; try `grove resume $(w.id)` after adopting, or coordinate a `grove handoff`"
end

"""Block arbitrary edits to a claimed `progress` W."""
function session_denial_progress_mutate(w::Node, eff::AbstractString)::Union{Nothing,String}
    w.kind !== :w && return nothing
    w.status !== :progress && return nothing
    !progress_has_session_record(w) && return nothing
    session_token_matches(w, eff) && return nothing
    return session_mutate_denied_message(w)
end

"""Pieces merged into journal `inv` before a `w` status transition (matches undo restore keys)."""
function session_journal_snap(w::Node)::Dict{String,Any}
    Dict{String,Any}(
        "had_session_before" => progress_has_session_record(w),
        "old_session" => get(w.attrs, "session", ""),
        "had_session_at_before" => haskey(w.attrs, "session_at"),
        "old_session_at" => get(w.attrs, "session_at", ""),
    )
end

"""Block leaving `progress` (set status, revert) unless owner or stale claim."""
function session_denial_progress_release(w::Node, eff::AbstractString)::Union{Nothing,String}
    w.kind !== :w && return nothing
    w.status !== :progress && return nothing
    !progress_has_session_record(w) && return nothing
    session_token_matches(w, eff) && return nothing
    session_claim_age_stale(w) && return nothing
    return session_release_denied_message(w)
end
