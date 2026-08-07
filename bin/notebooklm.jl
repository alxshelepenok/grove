#!/usr/bin/env julia

using Printf

const REPO = pwd()
const OUTPUT_FILE = joinpath(REPO, "context.txt")
const SRC_DIR = joinpath("packages", "grove", "src")

strip_images(content::AbstractString) = begin
    s = replace(content, "\r\n" => "\n")
    s = replace(s, r"<picture>.*?</picture>"s => "")
    s = replace(s, r"<a[^>]*><img[^>]*shields\.io[^>]*/>\s*</a>" => "")
    s = replace(s, r"\[!\[[^\]]*\]\([^)]*\)\]\([^)]*\)" => "")
    s = replace(s, r"<img[^>]*/?>" => "")
    s = replace(s, r"!\[[^\]]*\]\([^)]*\)" => "")
    s = replace(s, r"<a[^>]*>\s*</a>"s => "")
    s = replace(s, r"<p[^>]*>\s*</p>"s => "")
    return replace(s, r"\n{3,}" => "\n\n")
end

function reading_order(skills_dir::String)
    index_path = joinpath(skills_dir, "index.md")
    isfile(index_path) || return String[]
    ordered = String[]
    for line in eachline(index_path)
        m = match(r"^\d+\.\s+\[[^\]]*\]\(([^)]+)\)", line)
        m === nothing && continue
        link = m.captures[1]
        if endswith(link, "/")
            dir = joinpath(skills_dir, link)
            isdir(dir) || continue
            for f in sort(readdir(dir))
                endswith(f, ".md") && push!(ordered, joinpath(skills_dir, link, f))
            end
        else
            push!(ordered, joinpath(skills_dir, link))
        end
    end
    return ordered
end

function collect_md(root::String)
    files = String[]
    isdir(root) || return files
    for (dirpath, _, filenames) in walkdir(root)
        for filename in filenames
            endswith(filename, ".md") && push!(files, joinpath(dirpath, filename))
        end
    end
    return sort(files)
end

function read_text_file(path::String)
    try
        return read(path, String)
    catch err
        @warn "Skipping unreadable/non-UTF8 file" path error = err
        return nothing
    end
end

relative_to_repo(path::String) = replace(relpath(path, REPO), "\\" => "/")

function main()
    skills_dir = joinpath(REPO, "docs", "skills")
    seen = Set{String}()
    ordered = String[]

    push_unique!(p) = begin
        key = replace(p, "\\" => "/")
        key in seen && return
        push!(seen, key)
        push!(ordered, p)
    end

    push_unique!(joinpath(REPO, "README.md"))
    push_unique!(joinpath(REPO, "SECURITY.md"))

    push_unique!(joinpath(skills_dir, "index.md"))
    for p in reading_order(skills_dir)
        push_unique!(p)
    end
    for p in collect_md(skills_dir)
        push_unique!(p)
    end

    for p in collect_md(joinpath(REPO, "docs"))
        push_unique!(p)
    end

    src_files = String[]
    for (dirpath, _, filenames) in walkdir(joinpath(REPO, SRC_DIR))
        for filename in filenames
            endswith(filename, ".jl") && push!(src_files, joinpath(dirpath, filename))
        end
    end
    for p in sort(src_files)
        push_unique!(p)
    end

    open(OUTPUT_FILE, "w") do io
        for file_path in ordered
            content = read_text_file(file_path)
            content === nothing && continue
            rel_path = relative_to_repo(file_path)
            write(io, "<document path=\"$(rel_path)\">\n\n")
            write(io, strip(strip_images(content)))
            write(io, "\n\n</document>\n\n")
            @printf(" + added %s\n", rel_path)
        end
    end

    @info "Saved merged context" output = OUTPUT_FILE documents = length(ordered)
end

main()
