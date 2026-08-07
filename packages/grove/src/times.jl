"""RFC3339 UTC date-time second precision, e.g. `2026-05-05T14:03:22Z`."""
function utc_stamp_second()::String
    Dates.format(Dates.now(Dates.UTC), dateformat"yyyy-mm-ddTHH:MM:SS") * "Z"
end

_timestamp_blank(x)::Bool = isempty(x) || isempty(strip(String(x)))

function stamp_new_node!(n::Node)::Nothing
    t = utc_stamp_second()
    n.attrs["t_created"] = t
    n.attrs["t_updated"] = t
    nothing
end

function stamp_touch_node!(n::Node)::Nothing
    n.attrs["t_updated"] = utc_stamp_second()
    if _timestamp_blank(get(n.attrs, "t_created", ""))
        n.attrs["t_created"] = n.attrs["t_updated"]
    end
    nothing
end

function stamp_new_edge!(e::Edge)::Nothing
    if e.t_created === nothing || _timestamp_blank(e.t_created)
        e.t_created = utc_stamp_second()
    end
    nothing
end
