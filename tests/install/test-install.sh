#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

case $(uname -s) in
  Linux) os_part=linux ;;
  Darwin) os_part=macos ;;
  MINGW* | MSYS* | CYGWIN*) os_part=windows ;;
esac
case $(uname -m) in
  x86_64 | amd64) arch_part=x64 ;;
  arm64 | aarch64) arch_part=arm64 ;;
esac
target="${os_part}-${arch_part}"
if [ "$os_part" = windows ]; then ext=".exe"; else ext=""; fi

server="$work/server"
mkdir -p "$server"

printf '#!/usr/bin/env bash\necho fake grove\n' > "$server/grove$ext"
printf '#!/usr/bin/env bash\necho fake grove-mcp\n' > "$server/grove-mcp$ext"
mkdir -p "$server/dt/ui/views"
printf '#!/usr/bin/env bash\necho fake grove-desktop\n' > "$server/dt/grove-desktop$ext"
printf 'placeholder\n' > "$server/dt/ui/views/placeholder.hbs"
printf 'fake png\n' > "$server/dt/icon.png"
printf 'fake icns\n' > "$server/dt/icon.icns"
chmod +x "$server/grove$ext" "$server/grove-mcp$ext" "$server/dt/grove-desktop$ext"
(
  cd "$server"
  tar -czf "grove-v0.3.0-$target.tar.gz" "grove$ext"
  tar -czf "grove-mcp-v0.3.0-$target.tar.gz" "grove-mcp$ext"
  tar -czf "grove-desktop-v0.3.0-$target.tar.gz" -C dt "grove-desktop$ext" icon.png icon.icns ui
  rm "grove$ext" "grove-mcp$ext"
  rm -rf dt
  sha256sum *.tar.gz > "$server/SHA256SUMS" 2>/dev/null || {
    for f in *.tar.gz; do printf '%s  %s\n' "$(shasum -a 256 "$f" | cut -d' ' -f1)" "$f"; done > "$server/SHA256SUMS"
  }
)

openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$work/testkey.pem" 2>/dev/null
openssl pkey -in "$work/testkey.pem" -pubout -out "$work/testpub.pem" 2>/dev/null

build_manifest() {
  bin/manifest.sh --version 0.3.0 --sums "$server/SHA256SUMS" --artifacts-dir "$server" \
    --now "$1" --previous "$2" --output "$server/manifest.json"
  bin/sign.sh "$work/testkey.pem" "$server/manifest.json" "$server/manifest.json.sig"
}

run_install() {
  env GROVE_FETCH_ROOT="$server" GROVE_TRUSTED_KEY_FILE="$work/testpub.pem" HOME="$work/home$1" \
    bash install.sh --prefix "$work/inst$1" $2
}

build_manifest "$(date -u +%s)" /nonexistent

run_install 1 "" > "$work/out1" 2>&1
report $? "happy path installs both components"

grep -q "WARNING: GROVE_FETCH_ROOT is set" "$work/out1"
report $? "trust-override hooks print a loud warning"

[ -x "$work/inst1/bin/grove$ext" ] && [ -x "$work/inst1/bin/grove-mcp$ext" ]
report $? "binaries are executable"

[ -x "$work/inst1/grove-desktop/grove-desktop$ext" ] && [ -f "$work/inst1/grove-desktop/ui/views/placeholder.hbs" ]
report $? "desktop app installed with ui templates"

case $os_part in
  linux)
    grep -q "^Exec=$work/inst1/grove-desktop/grove-desktop$" "$work/home1/.local/share/applications/grove.desktop"
    report $? "linux launcher entry created"
    grep -q "^Icon=$work/inst1/grove-desktop/icon.png$" "$work/home1/.local/share/applications/grove.desktop" && [ -f "$work/inst1/grove-desktop/icon.png" ]
    report $? "linux launcher entry references the shipped icon"
    ;;
  macos)
    app="$work/home1/Applications/Grove.app"
    [ -x "$app/Contents/MacOS/launcher" ] && [ -f "$app/Contents/Info.plist" ]
    report $? "macos launcher app created"
    [ -f "$app/Contents/Resources/AppIcon.icns" ]
    report $? "macos launcher app installs the shipped icon"
    grep -q "<key>CFBundleIconFile</key>" "$app/Contents/Info.plist" && grep -q "<string>AppIcon</string>" "$app/Contents/Info.plist"
    report $? "macos launcher plist maps CFBundleIconFile to AppIcon"
    if command -v plutil >/dev/null 2>&1; then
      plutil -lint "$app/Contents/Info.plist" >/dev/null
      report $? "macos launcher plist passes plutil lint"
    fi
    ;;
esac

grep -q '^stable=1$' "$work/home1/.grove/.sequence"
report $? "sequence file written"

grep -qF "export PATH=\"$work/inst1/bin:\$PATH\"" "$work/home1/.profile"
report $? "PATH line added to .profile"

case $os_part in
  macos)
    grep -qF "export PATH=\"$work/inst1/bin:\$PATH\"" "$work/home1/.zshrc"
    report $? "PATH line added to .zshrc"
    grep -qF "export PATH=\"$work/inst1/bin:\$PATH\"" "$work/home1/.zprofile"
    report $? "PATH line added to .zprofile"
    ;;
esac

run_install 2 "--only grove-mcp" > "$work/out2" 2>&1
report $? "--only grove-mcp installs"

[ -x "$work/inst2/bin/grove-mcp$ext" ] && [ ! -e "$work/inst2/bin/grove$ext" ] && [ ! -e "$work/inst2/grove-desktop" ]
report $? "--only skips the other components"

[ ! -e "$work/home2/.local/share/applications/grove.desktop" ] && [ ! -e "$work/home2/Applications/Grove.app" ]
report $? "--only grove-mcp creates no launcher"

run_install 9 "--only grove-desktop" > "$work/out9" 2>&1
report $? "--only grove-desktop installs"

[ -x "$work/inst9/grove-desktop/grove-desktop$ext" ] && [ ! -e "$work/inst9/bin" ]
report $? "--only grove-desktop skips cli binaries"

[ ! -e "$work/home9/.profile" ]
report $? "--only grove-desktop does not touch PATH"

cp "$server/manifest.json" "$work/manifest.bak"
printf 'tampered' >> "$server/manifest.json"
if run_install 3 "" > "$work/out3" 2>&1; then r=1; else r=0; fi
report $r "tampered manifest rejected"
grep -q "signature verification failed" "$work/out3"
report $? "tamper error message is explicit"
cp "$work/manifest.bak" "$server/manifest.json"

build_manifest 1000000000 /nonexistent
if run_install 4 "" > "$work/out4" 2>&1; then r=1; else r=0; fi
report $r "expired manifest rejected"
grep -q "Manifest expired" "$work/out4"
report $? "expiry error message distinguishes staleness"

build_manifest "$(date -u +%s)" /nonexistent
mkdir -p "$work/home5/.grove"
printf 'format 1\nstable=9\n' > "$work/home5/.grove/.sequence"
if run_install 5 "" > "$work/out5" 2>&1; then r=1; else r=0; fi
report $r "rolled-back sequence rejected"

run_install 1 "" > "$work/out1b" 2>&1
report $? "same-sequence reinstall allowed"

[ "$(grep -cF "export PATH=\"$work/inst1/bin" "$work/home1/.profile")" -eq 1 ]
report $? "PATH line idempotent on reinstall"

case $os_part in
  macos)
    [ "$(grep -cF "export PATH=\"$work/inst1/bin" "$work/home1/.zshrc")" -eq 1 ] && [ "$(grep -cF "export PATH=\"$work/inst1/bin" "$work/home1/.zprofile")" -eq 1 ]
    report $? "zsh PATH lines idempotent on reinstall"
    ;;
esac

sed 's|https://github.com/alxshelepenok/grove/releases/download|https://evil.example.com/dl|' "$work/manifest.bak" > "$server/manifest.json"
bin/sign.sh "$work/testkey.pem" "$server/manifest.json" "$server/manifest.json.sig"
if run_install 6 "" > "$work/out6" 2>&1; then r=1; else r=0; fi
report $r "wrong-host artifact url rejected"

cp "$work/manifest.bak" "$server/manifest.json"
bin/sign.sh "$work/testkey.pem" "$server/manifest.json" "$server/manifest.json.sig"
printf 'corrupted' >> "$server/grove-v0.3.0-$target.tar.gz"
if run_install 7 "" > "$work/out7" 2>&1; then r=1; else r=0; fi
report $r "hash/size mismatch rejected"

env GROVE_ARTIFACT_URL="https://github.com/alxshelepenok/grove/releases/download/v0.3.0/grove-mcp-v0.3.0-$target.tar.gz" \
  GROVE_FETCH_ROOT="$server" HOME="$work/home8" \
  bash install.sh --prefix "$work/inst8" > "$work/out8" 2>&1
report $? "break-glass install works"
grep -q "WARNING: GROVE_ARTIFACT_URL break-glass" "$work/out8"
report $? "break-glass prints a loud warning"
[ ! -e "$work/home8/.grove/.sequence" ]
report $? "break-glass does not touch anti-rollback state"

bash install.sh --self-test > "$work/out9" 2>&1
report $? "install.sh --self-test passes"

if [ "$os_part" = macos ]; then
  mkdir -p "$work/dt2/ui/views"
  printf '#!/usr/bin/env bash\necho fake grove\n' > "$work/dt2/grove"
  printf '#!/usr/bin/env bash\necho fake grove-desktop\n' > "$work/dt2/grove-desktop"
  printf 'placeholder\n' > "$work/dt2/ui/views/placeholder.hbs"
  printf 'fake png\n' > "$work/dt2/icon.png"
  tar -czf "$server/grove-v0.3.0-$target.tar.gz" -C "$work/dt2" grove
  tar -czf "$server/grove-desktop-v0.3.0-$target.tar.gz" -C "$work/dt2" grove-desktop icon.png ui
  (
    cd "$server"
    sha256sum *.tar.gz > SHA256SUMS 2>/dev/null || {
      for f in *.tar.gz; do printf '%s  %s\n' "$(shasum -a 256 "$f" | cut -d' ' -f1)" "$f"; done > SHA256SUMS
    }
  )
  build_manifest "$(date -u +%s)" /nonexistent
  run_install 10 "" > "$work/out10" 2>&1
  report $? "icns-less archive installs"
  [ -x "$work/home10/Applications/Grove.app/Contents/MacOS/launcher" ]
  report $? "icns-less archive still yields a launcher"
  [ ! -e "$work/home10/Applications/Grove.app/Contents/Resources" ] && grep -q "keeps the default icon" "$work/out10"
  report $? "icns-less archive skips the icon with a notice"
fi

if [ "$fail" -gt 0 ]; then
  echo "=== captured installer outputs ==="
  for f in "$work"/out*; do
    echo "--- $f"
    cat "$f"
  done
fi

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
