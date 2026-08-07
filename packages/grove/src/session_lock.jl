const SESSION_POLL_SEC = 0.05
const SESSION_STALE_SEC = 60.0
const SESSION_HOLD_GRACE_SEC = 2.0
const SESSION_LOCK_MAX_SPIN = 6000

struct SessionExclusiveHold
    dir::String
end

struct SessionSharedHold
    dir::String
end

function session_locks_root(ctx::CliCtx)::String
    joinpath(abspath(ctx.root), ".grove", "locks")
end

"""If repo still has legacy `.grove.locks/` and `.grove/locks/` is absent, rename once."""
function maybe_migrate_legacy_session_locks!(ctx::CliCtx)::Nothing
    root_abs = abspath(ctx.root)
    legacy = joinpath(root_abs, ".grove.locks")
    fresh = joinpath(root_abs, ".grove", "locks")
    ispath(fresh) && return nothing
    ispath(legacy) || return nothing
    mkpath(joinpath(root_abs, ".grove"))
    try
        mv(legacy, fresh)
    catch
        # Concurrent agents or FS edge cases; new sessions still use `fresh`.
    end
    return nothing
end

session_exclusive_slot(ctx::CliCtx)::String =
    joinpath(session_locks_root(ctx), "exclusive")

session_readers_parent(ctx::CliCtx)::String =
    joinpath(session_locks_root(ctx), "readers")

session_holder(dir::AbstractString)::String =
    joinpath(string(dir), "holder")

"""Age of `holder` mtime vs `now`; `nothing` if file missing."""
function holder_age_sec(dir::AbstractString)::Union{Nothing,Float64}
    p = session_holder(dir)
    ispath(p) || return nothing
    time() - mtime(p)
end

"""Holder exists and younger than SESSION_STALE_SEC."""
function holder_fresh(dir::AbstractString)::Bool
    age = holder_age_sec(dir)
    age !== nothing && age <= SESSION_STALE_SEC
end

function exclusive_fresh_present(ctx::CliCtx)::Bool
    slot = session_exclusive_slot(ctx)
    isdir(slot) || return false
    holder_fresh(slot)
end

"""True if slot directory was created very recently (holder not written yet)."""
function holder_racing_grace(dir::AbstractString)::Bool
    isdir(dir) || return false
    time() - mtime(dir) < SESSION_HOLD_GRACE_SEC && !ispath(session_holder(dir))
end

function session_write_holder!(dir::AbstractString)::Nothing
    mkpath(dir)
    p = session_holder(dir)
    open(p, "w") do io
        println(io, getpid())
        println(io, utc_stamp_second())
    end
    nothing
end

function session_try_mkdir_exclusive(path::AbstractString)::Bool
    try
        mkdir(path)
        return true
    catch e
        (e isa Base.IOError || e isa Base.SystemError) && return false
        rethrow()
    end
end

function prune_stale_exclusive!(ctx::CliCtx)::Nothing
    slot = session_exclusive_slot(ctx)
    !isdir(slot) && return nothing
    holder_fresh(slot) && return nothing
    holder_racing_grace(slot) && return nothing
    holder_age_sec(slot) === nothing && time() - mtime(slot) < SESSION_HOLD_GRACE_SEC &&
        return nothing
    rm(slot, recursive=true, force=true)
    info(ctx,
        "warning: broke stale grove exclusive session lock (> " *
        string(Int(SESSION_STALE_SEC)) * " s or abandoned): " * slot)
    nothing
end

"""Remove stale per-reader dirs under `.grove/locks/readers/*/`."""
function prune_stale_readers!(ctx::CliCtx)::Nothing
    r = session_readers_parent(ctx)
    isdir(r) || return nothing
    for name in readdir(r)
        startswith(name, '.') && continue
        d = joinpath(r, name)
        isdir(d) || continue
        holder_fresh(d) && continue
        holder_racing_grace(d) && continue
        holder_age_sec(d) === nothing && time() - mtime(d) < SESSION_HOLD_GRACE_SEC &&
            continue
        rm(d, recursive=true, force=true)
        info(ctx, "warning: cleaned stale grove shared session lock: $(d)")
    end
    nothing
end

function count_fresh_readers(ctx::CliCtx)::Int
    r = session_readers_parent(ctx)
    isdir(r) || return 0
    n = 0
    for name in readdir(r)
        startswith(name, '.') && continue
        d = joinpath(r, name)
        isdir(d) || continue
        holder_fresh(d) && (n += 1)
    end
    n
end

struct SessionLockTimeoutError <: Exception
    msg::String
end

Base.showerror(io::IO, e::SessionLockTimeoutError) = print(io, e.msg)

"""Block until exclusive session lock acquired (writers wait for readers to drain)."""
function acquire_exclusive!(ctx::CliCtx)::SessionExclusiveHold
    maybe_migrate_legacy_session_locks!(ctx)
    root = session_locks_root(ctx)
    exc = session_exclusive_slot(ctx)
    rp = session_readers_parent(ctx)
    mkpath(root)
    mkpath(rp)

    spins = 0
    while true
        spins += 1
        spins > SESSION_LOCK_MAX_SPIN && throw(SessionLockTimeoutError(string(
            "timeout waiting for grove exclusive session lock (held ~>",
            round(Int, SESSION_LOCK_MAX_SPIN * SESSION_POLL_SEC),
            "s); try later")))

        prune_stale_exclusive!(ctx)
        prune_stale_readers!(ctx)

        exclusive_fresh_present(ctx) && (sleep(SESSION_POLL_SEC); continue)
        while count_fresh_readers(ctx) > 0
            sleep(SESSION_POLL_SEC)
        end
        for settle in 1:40
            count_fresh_readers(ctx) == 0 || break
            sleep(0.002)
        end
        count_fresh_readers(ctx) > 0 && continue

        session_try_mkdir_exclusive(exc) || (sleep(SESSION_POLL_SEC); continue)
        session_write_holder!(exc)

        if count_fresh_readers(ctx) > 0
            rm(exc, recursive=true, force=true)
            sleep(SESSION_POLL_SEC)
            continue
        end
        holder_fresh(exc) || (rm(exc, recursive=true, force=true); sleep(SESSION_POLL_SEC); continue)
        return SessionExclusiveHold(exc)
    end
end

"""Block until shared reader session acquired (exclusive writer must yield)."""
function acquire_shared!(ctx::CliCtx)::SessionSharedHold
    maybe_migrate_legacy_session_locks!(ctx)
    root = session_locks_root(ctx)
    rp = session_readers_parent(ctx)
    mkpath(root)
    mkpath(rp)

    spins = 0
    while true
        spins += 1
        spins > SESSION_LOCK_MAX_SPIN && throw(SessionLockTimeoutError(string(
            "timeout waiting for grove shared session lock (~>",
            round(Int, SESSION_LOCK_MAX_SPIN * SESSION_POLL_SEC),
            "s); try later")))

        prune_stale_exclusive!(ctx)

        exclusive_fresh_present(ctx) && (sleep(SESSION_POLL_SEC); continue)

        sid = string(getpid(), '-', time_ns(), '-', objectid(ctx) % 100_007)
        slot = joinpath(rp, sid)
        session_try_mkdir_exclusive(slot) || continue
        session_write_holder!(slot)

        exclusive_fresh_present(ctx) &&
            (rm(slot, recursive=true, force=true); sleep(SESSION_POLL_SEC); continue)
        holder_fresh(slot) || (rm(slot, recursive=true, force=true); sleep(SESSION_POLL_SEC); continue)
        return SessionSharedHold(slot)
    end
end

release_exclusive!(h::SessionExclusiveHold)::Nothing =
    (isdir(h.dir) && rm(h.dir, recursive=true, force=true); nothing)

release_shared!(h::SessionSharedHold)::Nothing =
    (isdir(h.dir) && rm(h.dir, recursive=true, force=true); nothing)

function with_session_exclusive(ctx::CliCtx, f::Function)::Int
    h = acquire_exclusive!(ctx)
    try
        return f()
    finally
        release_exclusive!(h)
    end
end

function with_session_shared(ctx::CliCtx, f::Function)::Int
    h = acquire_shared!(ctx)
    try
        return f()
    finally
        release_shared!(h)
    end
end
