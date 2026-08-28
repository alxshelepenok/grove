#!/usr/bin/env bash
set -euo pipefail

GROVE_REPO="alxshelepenok/grove"
GROVE_RAW_BASE="https://raw.githubusercontent.com/$GROVE_REPO/main"
GROVE_RELEASE_BASE="https://github.com/$GROVE_REPO/releases/download"
MINIMUM_SEQUENCE=1

read -r -d '' GROVE_TRUSTED_KEYS <<'PEM' || true
-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAq79TcasG8AcCaGWaUO4E
lmEjwmfL8TNk5dyZnCiWBZDM9XkFe830l6VuVoPk5kifBwjB+HAm7IZwXRA9lvMD
FTeavJpzo6NSyGpB4Tsu5t1pkvYllSh9oj5Hh+CiN80WE3qy+4e4DvnpGinqpjJk
Ulj2jnVLTqo5ZCR6b1cXS6PStXI47HNwxLCO5/3S7kJVd3ncUjxygrfozxTFWBTi
9mYC9zdt/kf+2T7xkdPW+RqhcQ3sxZogkNsaIUU+CePt349i1yWJlBbeejpPadJE
8/EXhXWizTcTqKh8+TrlBRq8VEQuL0G53ufpOJES7WPoc6xo6EG3XrTG0lHGAZCO
FQIDAQAB
-----END PUBLIC KEY-----
PEM

die() { echo "error: $*" >&2; exit 1; }
info() { echo "$*"; }

usage() {
  cat >&2 <<'EOF'
usage: install.sh [--channel stable] [--version X.Y.Z] [--only "grove grove-mcp grove-desktop"] [--prefix ~/.local/grove] [--self-test]

environment:
  GROVE_ARTIFACT_URL   break-glass: install this archive directly, skipping all verification
  GROVE_FETCH_ROOT     test hook: read files from this directory instead of HTTPS
  GROVE_TRUSTED_KEY_FILE  test hook: trust this public key instead of the embedded one
EOF
  exit 2
}

channel="stable"
version=""
only="grove grove-mcp grove-desktop"
prefix="$HOME/.local/grove"
self_test=0

while [ $# -gt 0 ]; do
  case $1 in
    --channel) channel=$2; shift 2 ;;
    --version) version=$2; shift 2 ;;
    --only) only=$2; shift 2 ;;
    --prefix) prefix=$2; shift 2 ;;
    --self-test) self_test=1; shift ;;
    *) usage ;;
  esac
done

echo "$channel" | grep -qE '^[a-z0-9_-]+$' || die "invalid channel name: $channel"

for hook in GROVE_TRUSTED_KEY_FILE GROVE_FETCH_ROOT GROVE_HOME; do
  eval "val=\${$hook:-}"
  if [ -n "$val" ]; then
    echo "WARNING: $hook is set - trust/store override active (test hook, not for production use)" >&2
  fi
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fetch() {
  if [ -n "${GROVE_FETCH_ROOT:-}" ]; then
    cp "$GROVE_FETCH_ROOT/$(basename "$1")" "$2" 2>/dev/null || die "fetch failed (test root): $1"
  else
    curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1" || die "could not download $1 - check your internet connection and try again"
  fi
}

epoch_of_iso() { date -u -d "$1" +%s 2>/dev/null || date -u -j -f "%Y-%m-%dT%H:%M:%SZ" "$1" +%s; }
sha256_of() { sha256sum "$1" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }
size_of() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1"; }

b64url_decode() {
  b64=$(tr -d '[:space:]' < "$1" | tr '_-' '/+')
  case $(( ${#b64} % 4 )) in
    0) ;;
    2) b64="$b64==" ;;
    3) b64="$b64=" ;;
    *) die "malformed base64url signature" ;;
  esac
  printf '%s' "$b64" | openssl base64 -d -A 2>/dev/null || die "signature is not valid base64"
}

verify_manifest() {
  pubkey="$work/trusted.pem"
  if [ -n "${GROVE_TRUSTED_KEY_FILE:-}" ]; then
    cp "$GROVE_TRUSTED_KEY_FILE" "$pubkey"
  else
    printf '%s\n' "$GROVE_TRUSTED_KEYS" > "$pubkey"
  fi
  b64url_decode "$2" > "$work/manifest.sig.bin"
  openssl dgst -sha256 -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:-1 \
    -verify "$pubkey" -signature "$work/manifest.sig.bin" "$1" >/dev/null 2>&1
}

extract_field() { sed -n "s/^  \"$2\": \"\\([^\"]*\\)\",\$/\\1/p" "$1"; }
extract_sequence() { sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$1"; }
extract_artifact() {
  block=$(sed -n "/^        \"$2\": {\$/,/^        }/p" "$1")
  [ -n "$block" ] || die "manifest has no artifact for $2"
  printf '%s\n' "$block" | sed -n "s/^          \"$3\": \\(.*\\)\$/\1/p" | sed -e 's/,$//' -e 's/^"//' -e 's/"$//'
}

read_sequence() {
  seqdir="${GROVE_HOME:-$HOME/.grove}"
  seqfile="$seqdir/.sequence"
  [ -f "$seqfile" ] || return 0
  tr -d '\r' < "$seqfile" | sed -n "s/^$1=\\([0-9][0-9]*\\)\$/\\1/p" | head -1
}

write_sequence() {
  seqdir="${GROVE_HOME:-$HOME/.grove}"
  seqfile="$seqdir/.sequence"
  mkdir -p "$seqdir"
  tmpseq="$work/sequence"
  if [ -f "$seqfile" ]; then
    grep -v "^$1=" "$seqfile" > "$tmpseq" || true
  else
    printf 'format 1\n' > "$tmpseq"
  fi
  printf '%s=%s\n' "$1" "$2" >> "$tmpseq"
  mv "$tmpseq" "$seqfile"
}

detect_target() {
  os=$(uname -s)
  arch=$(uname -m)
  case $os in
    Linux) os_part=linux ;;
    Darwin) os_part=macos ;;
    MINGW* | MSYS* | CYGWIN*) os_part=windows ;;
    *) die "unsupported operating system: $os" ;;
  esac
  case $arch in
    x86_64 | amd64) arch_part=x64 ;;
    arm64 | aarch64) arch_part=arm64 ;;
    *) die "unsupported architecture: $arch" ;;
  esac
  printf '%s_%s' "$os_part" "$arch_part"
}

install_archive() {
  archive="$1"
  comp="$2"
  mver="$3"
  target="$4"
  unpack="$work/unpack-$comp"
  mkdir -p "$unpack"
  tar -xzf "$archive" -C "$unpack" || die "failed to unpack $archive"
  if [ "$target" = windows_x64 ]; then binname="$comp.exe"; else binname="$comp"; fi
  binpath=$(find "$unpack" -type f -name "$binname" | head -1)
  [ -n "$binpath" ] || die "archive does not contain $binname"
  if [ "$comp" = grove-desktop ]; then
    dest="$prefix/grove-desktop"
    rm -rf "$dest"
    mkdir -p "$dest"
    cp -R "$unpack"/. "$dest"/
    [ -d "$dest/ui/views" ] || die "desktop archive is missing ui/views"
    chmod +x "$dest/$binname"
    info "installed grove-desktop ($mver) to $dest"
    return 0
  fi
  mkdir -p "$prefix/bin"
  cp "$binpath" "$prefix/bin/$binname"
  chmod +x "$prefix/bin/$binname"
  info "installed $binname ($mver) to $prefix/bin/$binname"
}

update_path_rc() {
  mkdir -p "$HOME"
  line="export PATH=\"$prefix/bin:\$PATH\""
  rcs="$HOME/.profile"
  [ -f "$HOME/.bashrc" ] && rcs="$rcs $HOME/.bashrc"
  [ -f "$HOME/.zshrc" ] && rcs="$rcs $HOME/.zshrc"
  for rc in $rcs; do
    if [ -f "$rc" ] && grep -qF "$line" "$rc"; then
      continue
    fi
    printf '%s # grove\n' "$line" >> "$rc"
    info "added $prefix/bin to PATH in $rc"
  done
  info "open a new shell or source your rc file to pick up the PATH change"
}

install_launcher() {
  case $1 in
    linux_*)
      appsdir="$HOME/.local/share/applications"
      mkdir -p "$appsdir"
      cat > "$appsdir/grove.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Grove
Comment=Grove desktop
Exec=$prefix/grove-desktop/grove-desktop
Icon=$prefix/grove-desktop/icon.png
Terminal=false
Categories=Development;Utility;
EOF
      info "created launcher entry $appsdir/grove.desktop"
      ;;
    macos_*)
      appdir="$HOME/Applications/Grove.app"
      rm -rf "$appdir"
      mkdir -p "$appdir/Contents/MacOS"
      cat > "$appdir/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>launcher</string>
  <key>CFBundleIdentifier</key>
  <string>com.grove.desktop</string>
  <key>CFBundleName</key>
  <string>Grove</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
</dict>
</plist>
EOF
      cat > "$appdir/Contents/MacOS/launcher" <<EOF
#!/bin/sh
exec "$prefix/grove-desktop/grove-desktop" "\$@"
EOF
      chmod +x "$appdir/Contents/MacOS/launcher"
      info "created launcher app $appdir"
      ;;
  esac
}

break_glass() {
  url="$GROVE_ARTIFACT_URL"
  echo "WARNING: GROVE_ARTIFACT_URL break-glass mode - skipping manifest, signature, and hash verification" >&2
  echo "WARNING: trust is delegated entirely to you and the channel that delivered this URL" >&2
  fetch "$url" "$work/breakglass.tar.gz"
  target=$(detect_target)
  case $(basename "$url") in
    grove-mcp-*) comp=grove-mcp ;;
    grove-desktop-*) comp=grove-desktop ;;
    *) comp=grove ;;
  esac
  install_archive "$work/breakglass.tar.gz" "$comp" break-glass "$target"
  info "break-glass install complete; anti-rollback state was not updated"
  exit 0
}

run_self_test() {
  info "running self-test in $work"
  openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$work/testkey.pem" 2>/dev/null
  openssl pkey -in "$work/testkey.pem" -pubout -out "$work/testpub.pem" 2>/dev/null

  mkdir -p "$work/server" "$work/fake-bin" "$work/fake-desktop/ui/views"
  printf '#!/usr/bin/env bash\necho fake grove\n' > "$work/fake-bin/grove"
  printf '#!/usr/bin/env bash\necho fake grove-mcp\n' > "$work/fake-bin/grove-mcp"
  printf '#!/usr/bin/env bash\necho fake grove-desktop\n' > "$work/fake-desktop/grove-desktop"
  printf 'placeholder\n' > "$work/fake-desktop/ui/views/placeholder.hbs"
  printf 'fake png\n' > "$work/fake-desktop/icon.png"
  printf 'fake icns\n' > "$work/fake-desktop/icon.icns"
  tar -czf "$work/server/grove-v9.9.9-selftest.tar.gz" -C "$work/fake-bin" grove
  tar -czf "$work/server/grove-mcp-v9.9.9-selftest.tar.gz" -C "$work/fake-bin" grove-mcp
  tar -czf "$work/server/grove-desktop-v9.9.9-selftest.tar.gz" -C "$work/fake-desktop" grove-desktop icon.png icon.icns ui

  make_manifest() {
    expires_iso=$(date -u -d "@$2" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -r "$2" +%Y-%m-%dT%H:%M:%SZ)
    cat > "$work/server/manifest.json" <<EOF
{
  "version": "9.9.9",
  "created_at": "2026-01-01T00:00:00Z",
  "expires_at": "$expires_iso",
  "channels": {
    "stable": {
      "sequence": $1,
      "artifacts": {
        "grove_selftest_x64": {
          "url": "$3/v9.9.9/grove-v9.9.9-selftest.tar.gz",
          "sha256": "$(sha256_of "$work/server/grove-v9.9.9-selftest.tar.gz")",
          "size": $(size_of "$work/server/grove-v9.9.9-selftest.tar.gz")
        },
        "grove_mcp_selftest_x64": {
          "url": "$3/v9.9.9/grove-mcp-v9.9.9-selftest.tar.gz",
          "sha256": "$(sha256_of "$work/server/grove-mcp-v9.9.9-selftest.tar.gz")",
          "size": $(size_of "$work/server/grove-mcp-v9.9.9-selftest.tar.gz")
        },
        "grove_desktop_selftest_x64": {
          "url": "$3/v9.9.9/grove-desktop-v9.9.9-selftest.tar.gz",
          "sha256": "$(sha256_of "$work/server/grove-desktop-v9.9.9-selftest.tar.gz")",
          "size": $(size_of "$work/server/grove-desktop-v9.9.9-selftest.tar.gz")
        }
      }
    }
  }
}
EOF
    openssl dgst -sha256 -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:-1 \
      -sign "$work/testkey.pem" -out "$work/sig.bin" "$work/server/manifest.json"
    { openssl base64 -A -in "$work/sig.bin" | tr '+/' '-_' | tr -d '='; echo; } > "$work/server/manifest.json.sig"
  }

  export GROVE_FETCH_ROOT="$work/server"
  export GROVE_TRUSTED_KEY_FILE="$work/testpub.pem"
  export HOME="$work/home"
  mkdir -p "$HOME"
  good_base="https://github.com/$GROVE_REPO/releases/download"
  now=$(date -u +%s)

  passes=0
  failures=0
  st_report() {
    if [ "$1" -eq 0 ]; then passes=$((passes+1)); echo "SELF-TEST PASS: $2"; else failures=$((failures+1)); echo "SELF-TEST FAIL: $2"; fi
  }

  make_manifest 7 $((now + 86400)) "$good_base"
  prefix="$work/inst1"
  detect_target() { printf 'selftest_x64'; }
  if ( main_install ) 2>"$work/err1"; then r=0; else r=1; fi
  st_report $r "happy path installs"
  [ -x "$work/inst1/bin/grove" ] && [ -x "$work/inst1/bin/grove-mcp" ]
  st_report $? "binaries installed and executable"
  [ -f "$work/inst1/grove-desktop/grove-desktop" ] && [ -d "$work/inst1/grove-desktop/ui/views" ]
  st_report $? "desktop app installed with ui templates"
  [ -f "$work/inst1/grove-desktop/icon.png" ]
  st_report $? "desktop archive ships the launcher icon"
  [ -f "$work/inst1/grove-desktop/icon.icns" ]
  st_report $? "desktop archive ships the launcher icns"

  make_manifest 7 $((now + 86400)) "$good_base"
  printf 'tampered' >> "$work/server/manifest.json"
  if ( main_install ) 2>/dev/null; then r=1; else r=0; fi
  st_report $r "tampered manifest rejected"

  make_manifest 7 $((now - 90000)) "$good_base"
  if ( main_install ) 2>/dev/null; then r=1; else r=0; fi
  st_report $r "expired manifest rejected"

  make_manifest 1 $((now + 86400)) "$good_base"
  printf 'stable=5\n' > "$HOME/.grove/.sequence" 2>/dev/null || { mkdir -p "$HOME/.grove"; printf 'stable=5\n' > "$HOME/.grove/.sequence"; }
  if ( main_install ) 2>/dev/null; then r=1; else r=0; fi
  st_report $r "rolled-back sequence rejected"

  make_manifest 7 $((now + 86400)) "https://evil.example.com/releases/download"
  if ( main_install ) 2>/dev/null; then r=1; else r=0; fi
  st_report $r "wrong-host artifact url rejected"

  echo "self-test: $passes passed, $failures failed"
  [ "$failures" -eq 0 ]
}

main_install() {
  target=$(detect_target)
  if [ -n "$version" ]; then
    manifest_url="$GROVE_RELEASE_BASE/v$version/manifest.json"
  else
    manifest_url="$GROVE_RAW_BASE/manifest.json"
  fi

  fetch "$manifest_url" "$work/manifest.json"
  fetch "$manifest_url.sig" "$work/manifest.json.sig"

  verify_manifest "$work/manifest.json" "$work/manifest.json.sig" \
    || die "manifest signature verification failed - refusing to parse or install"

  m_version=$(extract_field "$work/manifest.json" version)
  m_expires=$(extract_field "$work/manifest.json" expires_at)
  m_sequence=$(extract_sequence "$work/manifest.json")
  [ -n "$m_version" ] && [ -n "$m_expires" ] && [ -n "$m_sequence" ] || die "manifest is missing required fields"
  if [ -n "$version" ] && [ "$m_version" != "$version" ]; then
    die "manifest version $m_version does not match requested $version"
  fi

  now=$(date -u +%s)
  expires_epoch=$(epoch_of_iso "$m_expires") || die "cannot parse expires_at: $m_expires"
  if [ "$now" -gt $((expires_epoch + 86400)) ]; then
    die "Manifest expired. A new release is pending; try again later."
  fi

  stored=$(read_sequence "$channel")
  if [ -n "$stored" ] && [ "$m_sequence" -lt "$stored" ]; then
    die "manifest sequence $m_sequence is older than installed sequence $stored - possible rollback, refusing"
  fi
  if [ "$m_sequence" -lt "$MINIMUM_SEQUENCE" ]; then
    die "manifest sequence $m_sequence is below the minimum $MINIMUM_SEQUENCE"
  fi

  for comp in $only; do
    key=$(printf '%s_%s' "$comp" "$target" | tr '-' '_')
    url=$(extract_artifact "$work/manifest.json" "$key" url)
    expect_sha=$(extract_artifact "$work/manifest.json" "$key" sha256)
    expect_size=$(extract_artifact "$work/manifest.json" "$key" size)
    case $url in
      "$GROVE_RELEASE_BASE"/v"$m_version"/*) ;;
      *) die "artifact URL for $key is not on the allowed host: $url" ;;
    esac
    fetch "$url" "$work/$comp.tar.gz"
    actual_size=$(size_of "$work/$comp.tar.gz")
    [ "$actual_size" = "$expect_size" ] || die "size mismatch for $key: expected $expect_size, got $actual_size"
    actual_sha=$(sha256_of "$work/$comp.tar.gz")
    [ "$actual_sha" = "$expect_sha" ] || die "sha256 mismatch for $key - the downloaded bytes do not match the signed manifest"
    install_archive "$work/$comp.tar.gz" "$comp" "$m_version" "$target"
  done

  case " $only " in
    *" grove-desktop "*) install_launcher "$target" ;;
  esac
  case " $only " in
    *" grove "* | *" grove-mcp "*) update_path_rc ;;
  esac

  write_sequence "$channel" "$m_sequence"
  info "grove $m_version installed (channel $channel, sequence $m_sequence)"
  info "verify any artifact again with: bin/verify.sh docs/security/artifacts/public-keys/grove-manifest-2026-08.pem <file> <file.sig>"
}

if [ "${GROVE_ARTIFACT_URL:-}" != "" ]; then
  break_glass
fi

if [ "$self_test" -eq 1 ]; then
  run_self_test
  exit $?
fi

main_install
