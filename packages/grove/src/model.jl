const NODE_KINDS = (:g, :w, :d, :q, :b, :t, :y, :a)
const EDGE_LABELS = (
    :blocks, :causes, :implements, :asks, :tests, :supersedes, :targets, :produces,
    :distills,
)

const STATUS = Dict(
    :g => (:unverified, :partial, :verified, :declined),
    :w => (:proposed, :ready, :progress, :done, :rejected, :archived),
    :d => (:proposed, :accepted, :rejected, :superseded),
    :q => (:open, :deferred, :answered, :dropped),
    :b => (:proposed, :testing, :validated, :invalidated_acceptable, :invalidated_blocking),
    :t => (:open, :done),
    :y => (:proposed, :active, :stale, :superseded),
    :a => (:present,),
)

const W_TYPES = (:feature, :refactor, :bug, :spike)
const CYNEFIN = (:clear, :complicated, :complex, :chaotic)

function isterminal(kind::Symbol, status::Symbol)::Bool
    if kind === :w
        return status in (:done, :rejected, :archived)
    elseif kind === :g
        return status in (:verified, :declined)
    elseif kind === :d
        return status in (:accepted, :rejected, :superseded)
    elseif kind === :q
        return status in (:answered, :deferred, :dropped)
    elseif kind === :b
        return status in (:validated, :invalidated_acceptable, :invalidated_blocking)
    elseif kind === :t
        return status === :done
    elseif kind === :y
        return status === :superseded
    elseif kind === :a
        return false
    end
    return false
end

mutable struct Node
    kind::Symbol
    id::String
    title::String
    type::Union{Symbol,Nothing}
    status::Symbol
    cynefin::Union{Symbol,Nothing}
    attrs::Dict{String,String}
    fields::Dict{Symbol,Any}
    archived::Bool
end

Node(kind, id; title="", type=nothing, status=:proposed, cynefin=nothing) =
    Node(kind, id, title, type, status, cynefin, Dict{String,String}(), Dict{Symbol,Any}(), false)

"""Strict clearance for `(predecessor, blocks, ·)` edges: terminal+ on goals ⇒ `verified` (not merely `declined`)."""
clears_blocks_predecessor(p::Node)::Bool =
    (p.kind === :g ? p.status === :verified : isterminal(p.kind, p.status))

mutable struct Edge
    from::String
    label::Symbol
    to::String
    t_created::Union{Nothing,String}
end

Edge(from::AbstractString, label::Symbol, to::AbstractString) =
    Edge(String(from), label, String(to), nothing)

mutable struct State
    nodes::Dict{String,Node}
    edges::Vector{Edge}
    counters::Dict{Char,Int}
    raw_comments::Vector{String}
    """Additive step between successive allocated numeric suffixes (default 1)."""
    id_stride::Int
    """First numeric suffix issued for each empty family lane (lane-based allocation)."""
    id_offset::Int
    """Minimum digit width when formatting new IDs (`lpad`; grows if number is larger)."""
    id_pad_width::Int
end

State() = State(Dict{String,Node}(), Edge[], Dict{Char,Int}(), String[], 1, 1, 2)

idfamily(id::AbstractString) = id[1]

function getnode(st::State, id::AbstractString)
    n = get(st.nodes, String(id), nothing)
    n === nothing && error("not found: $id")
    n
end

out_edges(st::State, id::AbstractString, label::Symbol) =
    Iterators.filter(e -> e.from == id && e.label === label, st.edges)
in_edges(st::State, id::AbstractString, label::Symbol) =
    Iterators.filter(e -> e.to == id && e.label === label, st.edges)

function listnodes(st::State, kind::Symbol; include_archived::Bool=false)
    out = Node[]
    for n in values(st.nodes)
        n.kind === kind || continue
        (include_archived || !n.archived) || continue
        push!(out, n)
    end
    sort!(out; by=n -> n.id)
    out
end
