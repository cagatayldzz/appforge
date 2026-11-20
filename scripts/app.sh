#!/bin/bash
# Usage:
#   scripts/app.sh --dev                     debug build, then launch dist/AppForge.app
#   scripts/app.sh [debug|release] --build   build dist/AppForge.app (default: debug)
#   scripts/app.sh --check                   scripts and their tests, formatting, clippy and tests
#   scripts/app.sh --update                  upgrade workspace dependency requirements and the lockfile, then run checks

set -euo pipefail

# Project paths and helpers.
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"

usage="Usage: scripts/app.sh --dev | [debug|release] --build | --check | --update | --icon"
resources="$project_dir/apps/appforge/macos"
app_dir="$project_dir/dist/AppForge.app"

fail() {
  echo "$1" >&2
  exit 1
}

# Parse arguments and verify the toolchain.
configuration="debug"
action=""

for argument in "$@"; do
  case "$argument" in
    debug | release) configuration="$argument" ;;
    --dev) action="dev"; configuration="debug" ;;
    --build) action="build" ;;
    --check) action="check" ;;
    --update) action="update" ;;
    --icon) action="icon" ;;
    *) fail "$usage" ;;
  esac
done

[[ -n "$action" ]] || fail "$usage"
command -v cargo >/dev/null 2>&1 || fail "Rust was not found. Install the Rust toolchain and try again."

# Formatting, clippy and tests.
run_check() {
  local script
  for script in scripts/*.sh scripts/tests/*.sh; do
    bash -n "$script"
  done
  bash scripts/tests/run.sh
  cargo fmt --all -- --check
  cargo clippy --locked --workspace --all-targets -- -D warnings
  cargo test --locked --workspace
}


# Keep the manifests and lockfile together if an upgrade or check fails.
run_update() (
  cargo upgrade --version >/dev/null 2>&1 || fail 'Cargo Upgrade is required. Install it with: cargo install cargo-edit --locked'
  files=(Cargo.toml Cargo.lock apps/*/Cargo.toml crates/*/Cargo.toml)
  backup="$(mktemp -d)"
  restore=false
  cleanup_update() {
    status=$?
    trap - EXIT
    if [[ "$restore" == true ]]; then
      for file in "${files[@]}"; do
        if ! cp -p "$backup/$file" "$file"; then
          fail "Could not restore $file. Dependency backups are in $backup."
        fi
      done
      echo "Dependency update failed. The original manifests and lockfile were restored." >&2
    fi
    rm -rf "$backup"
    exit "$status"
  }
  trap cleanup_update EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  for file in "${files[@]}"; do
    mkdir -p "$backup/$(dirname "$file")"
    cp -p "$file" "$backup/$file"
  done
  restore=true
  cargo upgrade --compatible allow --incompatible allow --pinned ignore
  cargo update
  run_check
  restore=false
  echo 'Dependencies updated and verified. Review the manifests and Cargo.lock with git diff.'
)

# Render icon.svg at every size macOS expects and pack it into AppIcon.icns.
make_icon() {
  local work iconset
  work="$(mktemp -d)"
  iconset="$work/AppIcon.iconset"
  mkdir "$iconset"
  # Headless Chrome keeps the transparent corners (Quick Look flattens them to white).
  local chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
  [[ -x "$chrome" ]] || fail "Google Chrome is required to render icon.svg."
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --default-background-color=00000000 \
    --window-size=1024,1024 --screenshot="$work/icon.svg.png" "file://$resources/icon.svg" >/dev/null 2>&1
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$work/icon.svg.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$work/icon.svg.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$resources/AppIcon.icns"
  # Draw the development ribbon over the same rendered icon without a Swift toolchain.
  {
    printf '%s' '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024"><image width="1024" height="1024" href="data:image/png;base64,'
    base64 < "$work/icon.svg.png" | tr -d '\n'
    cat <<'SVG'
"/><defs><clipPath id="icon"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath></defs>
<g clip-path="url(#icon)"><path d="M100 500V100H500Z" fill="#f26133"/>
<text x="238" y="317" transform="rotate(-45 238 317)" fill="white" font-family="-apple-system, sans-serif" font-size="90" font-weight="bold">DEV</text></g></svg>
SVG
  } > "$work/dev.svg"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --default-background-color=00000000 \
    --window-size=1024,1024 --screenshot="$work/dev.png" "file://$work/dev.svg" >/dev/null 2>&1
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$work/dev.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$work/dev.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$resources/DevIcon.icns"
  rm -rf "$work"
  echo "Ready: $resources/AppIcon.icns and $resources/DevIcon.icns"
}

# Build the binary, wrap it in dist/AppForge.app and sign it.
compile() {
  local profile_flags=()
  [[ "$configuration" == release ]] && profile_flags=(--release)

  cargo build --locked ${profile_flags[@]+"${profile_flags[@]}"} -p appforge
}

bundle() {
  rm -rf "$app_dir"
  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  cp "$project_dir/target/$configuration/AppForge" "$app_dir/Contents/MacOS/AppForge"
  cp "$resources/Info.plist" "$app_dir/Contents/Info.plist"
  if [[ "$configuration" == debug ]]; then
    cp "$resources/DevIcon.icns" "$app_dir/Contents/Resources/DevIcon.icns"
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile DevIcon' "$app_dir/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.cagatayldzz.appforge.dev' "$app_dir/Contents/Info.plist"
  else
    cp "$resources/AppIcon.icns" "$app_dir/Contents/Resources/AppIcon.icns"
  fi
}

# Local development signature. Distribution requires Developer ID + notarization.
# An ad-hoc signature ("-") is new on every build, so macOS asks for the Keychain password again after each build.
# A certificate that stays the same keeps "Always Allow" working: create a local code-signing certificate named
# "AppForge Dev" in Keychain Access (Certificate Assistant > Create a Certificate > Code Signing), or name another
# one in APPFORGE_SIGN_IDENTITY.
sign() {
  local output identity="${APPFORGE_SIGN_IDENTITY:-AppForge Dev}"
  if ! security find-identity -p codesigning 2>/dev/null | grep -qF "\"$identity\""; then
    [[ -z "${APPFORGE_SIGN_IDENTITY:-}" ]] || fail "No code-signing certificate named \"$identity\" was found in your keychains."
    echo "No \"$identity\" certificate found: signing ad hoc, so the Keychain asks for its password after every build." >&2
    identity="-"
  fi
  output="$(codesign --force --sign "$identity" "$app_dir" 2>&1)" || fail "$output"
}

# `open` only brings an already running instance forward, so quit the running development app first: politely, then by force.
stop_running_app() {
  local identifier="com.cagatayldzz.appforge.dev" attempt
  pgrep -f "$app_dir/Contents/MacOS/AppForge" >/dev/null 2>&1 || return 0
  osascript -e "tell application id \"$identifier\" to quit" >/dev/null 2>&1 || true
  for attempt in 1 2 3 4 5 6 7 8 9 10; do
    pgrep -f "$app_dir/Contents/MacOS/AppForge" >/dev/null 2>&1 || return 0
    sleep 0.3
  done
  pkill -f "$app_dir/Contents/MacOS/AppForge" >/dev/null 2>&1 || true
  sleep 0.5
}

build_app() {
  compile
  bundle
  sign
  echo "Ready: $app_dir"
}

# Run the requested action.
case "$action" in
  check) run_check ;;
  update) run_update ;;
  icon) make_icon ;;
  build) build_app ;;
  dev)
    compile
    stop_running_app
    bundle
    sign
    echo "Ready: $app_dir"
    open "$app_dir"
    ;;
esac
