#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

[ $# -ge 2 ] && [ $# -le 3 ] || { echo "usage: sign.sh <private-key.pem> <file> [output.sig]" >&2; exit 2; }

key=$1
file=$2
out=${3:-"$file.sig"}

[ -f "$key" ] || die "private key not found: $key"
[ -f "$file" ] || die "file not found: $file"

raw=$(mktemp)
trap 'rm -f "$raw"' EXIT

openssl dgst -sha256 -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:-1 -sign "$key" -out "$raw" "$file"
{ openssl base64 -A -in "$raw" | tr '+/' '-_' | tr -d '='; echo; } > "$out"
