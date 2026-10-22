.PHONY: test coverage fmt-md check-md

test:
	julia --project=packages/grove -e "using Pkg; Pkg.test()"

coverage:
	julia --project=packages/grove -e "using Pkg; Pkg.test(; coverage=true)"
	julia --project=packages/grove -e "ENV[\"COVERAGE_MIN_PCT\"]=\"70\"; include(\"bin/coverage/summary.jl\")"

notebooklm:
	julia --project=. bin/notebooklm.jl

fmt-md:
	bash bin/md-format.sh

check-md:
	bash bin/md-format.sh --check