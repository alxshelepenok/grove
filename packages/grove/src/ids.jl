const FAMILY_PREFIX = Dict(
    :g => 'G', :w => 'W', :d => 'D', :q => 'Q', :b => 'B', :t => 'T',
    :y => 'Y', :a => 'A',
)

const ID_WITH_NUM_REGEX = r"^[A-Z]-(0*[1-9][0-9]*)$"

"""Parse uppercase family letter and numeric suffix (leading zeros ignored)."""
function parse_id_numeric(id::AbstractString)::Tuple{Char, Int}
    s = String(strip(id))
    m = match(ID_WITH_NUM_REGEX, s)
    m === nothing && error("malformed id: $id")
    prefix = Char(s[firstindex(s)])
    suf = split(s, '-'; limit = 2)[2]
    n = parse(Int, suf)
    prefix, n
end

"""Pad numeric suffix (`lpad`; width grows past `min_pad` when needed)."""
function format_allocated_id(prefix::Char, numeric::Int, min_pad::Int)::String
    w = max(2, Int(min_pad), ndigits(numeric))
    string(prefix, '-', lpad(string(numeric), w, '0'))
end

"""
Allocate next ID for kind. Uses `state.counters` as the numeric high-water mark
per family (`record_id!`). With `id_stride=1` / `id_offset=1` behaves like legacy `(max+1)`.
Otherwise the first suffix is `offset`, then `prior_max + stride`.
"""
function next_id!(st::State, kind::Symbol)::String
    stride = max(1, st.id_stride)
    off = max(1, st.id_offset)
    prefix = FAMILY_PREFIX[kind]
    cur = get(st.counters, prefix, 0)
    nextnum = cur <= 0 ? off : cur + stride
    st.counters[prefix] = nextnum
    format_allocated_id(prefix, nextnum, st.id_pad_width)
end

function record_id!(st::State, id::AbstractString)
    isempty(id) && return
    p, n = parse_id_numeric(id)
    cur = get(st.counters, p, 0)
    if n > cur
        st.counters[p] = n
    end
end
