function treewidth_fill_missing(nbrs::Set{String}, adj::Dict{String,Set{String}})::Int
    c = 0
    for x in nbrs, y in nbrs
        x < y || continue
        y in adj[x] || (c += 1)
    end
    c
end

function treewidth_upper(st::State)::Int
    adj = Dict{String,Set{String}}()
    for (id, n) in st.nodes
        n.archived && continue
        adj[id] = Set{String}()
    end
    for e in st.edges
        e.from == e.to && continue
        (haskey(adj, e.from) && haskey(adj, e.to)) || continue
        push!(adj[e.from], e.to)
        push!(adj[e.to], e.from)
    end
    width = 0
    while !isempty(adj)
        pick = first(sort!(collect(keys(adj)); by=id -> (treewidth_fill_missing(adj[id], adj), id)))
        nbrs = sort!(collect(adj[pick]))
        width = max(width, length(nbrs))
        for i in eachindex(nbrs), j in i+1:length(nbrs)
            push!(adj[nbrs[i]], nbrs[j])
            push!(adj[nbrs[j]], nbrs[i])
        end
        for x in nbrs
            delete!(adj[x], pick)
        end
        delete!(adj, pick)
    end
    width
end
