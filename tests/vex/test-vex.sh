#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

run() { julia --project=packages/grove bin/validate-vex.jl "$1" >/dev/null 2>&1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

run docs/security/artifacts/vex.json
report $? "committed vex.json validates"

cat > "$work/valid.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 2,
  "vulnerabilities": [
    {
      "id": "RUSTSEC-2024-0421",
      "affects": [{"ref": "pkg:cargo/idna@0.5.0"}],
      "analysis": {
        "state": "not_affected",
        "justification": "code_not_reachable",
        "detail": "grove-mcp does not construct URLs from untrusted input"
      }
    }
  ]
}
EOF
run "$work/valid.json"
report $? "valid not_affected statement accepted"

cat > "$work/no-justification.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "vulnerabilities": [
    {
      "id": "CVE-2024-1234",
      "affects": [{"ref": "pkg:cargo/serde@1.0.0"}],
      "analysis": {"state": "not_affected"}
    }
  ]
}
EOF
if run "$work/no-justification.json"; then r=1; else r=0; fi
report $r "not_affected without justification or detail rejected"

cat > "$work/bad-state.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "vulnerabilities": [
    {
      "id": "JLSEC-2025-1",
      "affects": [{"ref": "pkg:julia/JSON@0.21.4"}],
      "analysis": {"state": "ignored"}
    }
  ]
}
EOF
if run "$work/bad-state.json"; then r=1; else r=0; fi
report $r "unknown analysis state rejected"

cat > "$work/bad-id.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "vulnerabilities": [
    {
      "id": "SOMETHING-1",
      "affects": [{"ref": "pkg:cargo/serde@1.0.0"}],
      "analysis": {"state": "under_investigation"}
    }
  ]
}
EOF
if run "$work/bad-id.json"; then r=1; else r=0; fi
report $r "malformed advisory id rejected"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
