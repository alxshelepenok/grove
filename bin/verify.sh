#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

[ $# -eq 3 ] || { echo "usage: verify.sh <public-key.pem> <file> <signature.sig>" >&2; exit 2; }

pub=$1
file=$2
sig=$3

[ -f "$pub" ] || die "public key not found: $pub"
[ -f "$file" ] || die "file not found: $file"
[ -f "$sig" ] || die "signature not found: $sig"

b64=$(tr -d '[:space:]' < "$sig" | tr '_-' '/+')
case $(( ${#b64} % 4 )) in
  0) ;;
  2) b64="$b64==" ;;
  3) b64="$b64=" ;;
  *) die "malformed base64url signature" ;;
esac

raw=$(mktemp)
trap 'rm -f "$raw"' EXIT
printf '%s' "$b64" | openssl base64 -d -A -out "$raw" 2>/dev/null || die "signature is not valid base64"

if openssl dgst -sha256 -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:-1 -verify "$pub" -signature "$raw" "$file" >/dev/null 2>&1; then
  echo "OK"
else
  die "signature verification failed"
fi
