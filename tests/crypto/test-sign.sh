#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

sign=bin/sign.sh
verify=bin/verify.sh

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

openssl version

openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$work/key-a.pem" 2>/dev/null
openssl pkey -in "$work/key-a.pem" -pubout -out "$work/pub-a.pem" 2>/dev/null
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$work/key-b.pem" 2>/dev/null
openssl pkey -in "$work/key-b.pem" -pubout -out "$work/pub-b.pem" 2>/dev/null
echo '{"version":"0.1.0","channel":"stable"}' > "$work/manifest.json"

"$sign" "$work/key-a.pem" "$work/manifest.json" "$work/good.sig"
"$verify" "$work/pub-a.pem" "$work/manifest.json" "$work/good.sig" >/dev/null 2>&1
report $? "known-good signature verifies"

if [ -n "$(tr -d 'A-Za-z0-9_\n-' < "$work/good.sig")" ]; then r=1; else r=0; fi
report $r "signature is base64url without padding"

cp "$work/manifest.json" "$work/tampered.json"
printf 'x' >> "$work/tampered.json"
if "$verify" "$work/pub-a.pem" "$work/tampered.json" "$work/good.sig" >/dev/null 2>&1; then r=1; else r=0; fi
report $r "tampered file rejected"

if "$verify" "$work/pub-b.pem" "$work/manifest.json" "$work/good.sig" >/dev/null 2>&1; then r=1; else r=0; fi
report $r "wrong key rejected"

size=$(wc -c < "$work/good.sig")
head -c $((size - 8)) "$work/good.sig" > "$work/truncated.sig"
if "$verify" "$work/pub-a.pem" "$work/manifest.json" "$work/truncated.sig" >/dev/null 2>&1; then r=1; else r=0; fi
report $r "truncated signature rejected"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
