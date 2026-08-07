#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

trivy=${TRIVY_BIN:-trivy}
vex_default=docs/security/artifacts/vex.json

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

mkdir -p "$work/fixture"
cat > "$work/fixture/Project.toml" <<'EOF'
name = "w76fixture"
uuid = "11111111-1111-1111-1111-111111111111"
version = "0.1.0"

[deps]
HTTP = "cd3eb016-35fb-5094-929b-558a96fad6f3"
EOF
cat > "$work/fixture/Manifest.toml" <<'EOF'
# This file is machine-generated - editing it directly is not advised

julia_version = "1.12.6"
manifest_format = "2.0"
project_hash = "0000000000000000000000000000000000000000000000000000000000000000"

[[deps.HTTP]]
uuid = "cd3eb016-35fb-5094-929b-558a96fad6f3"
version = "1.10.15"
EOF

bin/audit.sh --trivy-bin "$trivy" --targets "$work/fixture" > "$work/scan.out" 2>&1
rc=$?
[ $rc -ne 0 ]
report $? "fixture with HTTP.jl 1.10.15 is rejected"

grep -q "CVE-2025-52479" "$work/scan.out"
report $? "known advisory CVE-2025-52479 appears in findings"

grep -q "warning: UNKNOWN JLSEC-" "$work/scan.out"
report $? "UNKNOWN-severity JLSEC advisories are reported as warnings, not dropped"

cat > "$work/vex.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "vulnerabilities": [
    {
      "id": "CVE-2025-52479",
      "affects": [{"ref": "pkg:julia/HTTP@1.10.15"}],
      "analysis": {
        "state": "not_affected",
        "justification": "code_not_reachable",
        "detail": "suppression test fixture"
      }
    }
  ]
}
EOF

bin/audit.sh --trivy-bin "$trivy" --vex "$work/vex.json" --targets "$work/fixture" > "$work/scan-vex.out" 2>&1
if grep -q "CVE-2025-52479" "$work/scan-vex.out"; then r=1; else r=0; fi
report $r "VEX not_affected statement suppresses the finding"

sed 's/pkg:julia\/HTTP@1.10.15/pkg:julia\/HTTP@9.9.9/' "$work/vex.json" > "$work/vex-wrongver.json"
bin/audit.sh --trivy-bin "$trivy" --vex "$work/vex-wrongver.json" --targets "$work/fixture" > "$work/scan-wrongver.out" 2>&1
grep -q "CVE-2025-52479" "$work/scan-wrongver.out"
report $? "VEX statement for a different version does not suppress"

cat > "$work/report.json" <<'EOF'
{
  "Results": [
    {
      "Target": "fixture",
      "Vulnerabilities": [
        {"VulnerabilityID": "CVE-2099-0001", "PkgName": "HTTP", "InstalledVersion": "1.10.15", "Severity": "HIGH"},
        {"VulnerabilityID": "CVE-2099-0001", "PkgName": "Other", "InstalledVersion": "1.10.15", "Severity": "HIGH"},
        {"VulnerabilityID": "CVE-2099-0002", "PkgName": "HTTP", "InstalledVersion": "1.10.15-beta", "Severity": "HIGH"},
        {"VulnerabilityID": "CVE-2099-0003", "PkgName": "HTTP", "InstalledVersion": "1.10.15", "Severity": "UNKNOWN"}
      ]
    }
  ]
}
EOF
cat > "$work/vex-exact.json" <<'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "vulnerabilities": [
    {
      "id": "CVE-2099-0001",
      "affects": [{"ref": "pkg:julia/HTTP@1.10.15"}],
      "analysis": {"state": "not_affected", "justification": "code_not_reachable", "detail": "fixture"}
    }
  ]
}
EOF
julia --project=packages/grove bin/audit-filter.jl "$work/report.json" "$work/vex-exact.json" > "$work/filter.out" 2>&1
rc=$?
[ $rc -ne 0 ]
report $? "exact-match filter still fails on unsuppressed findings"

grep -q "Other@1.10.15" "$work/filter.out"
report $? "same version in a different package is not suppressed"

grep -q "HTTP@1.10.15-beta" "$work/filter.out"
report $? "different version of the same package is not suppressed"

if grep -q "HIGH CVE-2099-0001 HTTP@1.10.15 " "$work/filter.out"; then r=1; else r=0; fi
report $r "exact name+version match is suppressed"

grep -q "warning: UNKNOWN CVE-2099-0003" "$work/filter.out"
report $? "UNKNOWN severity maps to a non-blocking warning"

bin/audit.sh --trivy-bin "$trivy" --vex "$vex_default" > "$work/scan-repo.out" 2>&1
rc=$?
if [ $rc -ne 0 ]; then cat "$work/scan-repo.out"; fi
report $rc "repository lockfiles pass the audit"

bin/audit.sh --trivy-bin /nonexistent/trivy --targets Cargo.lock > /dev/null 2>&1
report $? "scanner outage fails open by default"

if bin/audit.sh --trivy-bin /nonexistent/trivy --fail-closed --targets Cargo.lock > /dev/null 2>&1; then r=1; else r=0; fi
report $r "scanner outage fails closed with --fail-closed"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
