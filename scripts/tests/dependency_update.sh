#!/bin/bash
# The update command keeps successful upgrades and restores every file after a failed check.
set -euo pipefail
cd "$(dirname "$0")/../.."

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch"/{scripts,apps/example,crates/example,bin}
cp scripts/app.sh "$scratch/scripts/app.sh"
cat > "$scratch/bin/cargo" <<'CARGO'
#!/bin/bash
set -euo pipefail
case "$1" in
  upgrade)
    [[ "${2:-}" != --version ]] || exit 0
    [[ "$*" == 'upgrade --compatible allow --incompatible allow --pinned ignore' ]]
    printf 'upgraded\n' >> Cargo.toml
    printf 'upgraded\n' >> apps/example/Cargo.toml
    printf 'upgraded\n' >> crates/example/Cargo.toml
    ;;
  update) printf 'resolved\n' >> Cargo.lock ;;
  test) [[ "${TEST_UPDATE_FAIL:-}" != true ]] ;;
esac
CARGO
chmod +x "$scratch/bin/cargo"
cd "$scratch"
export PATH="$scratch/bin:$PATH"
files=(Cargo.toml Cargo.lock apps/example/Cargo.toml crates/example/Cargo.toml)
for file in "${files[@]}"; do
  printf 'existing edit\n' > "$file"
done
mkdir -p scripts/tests
printf '#!/bin/bash\nexit 0\n' > scripts/tests/run.sh

if TEST_UPDATE_FAIL=true bash scripts/app.sh --update > failed.log 2>&1; then
  echo 'An update with failing tests unexpectedly succeeded.' >&2
  exit 1
fi
for file in "${files[@]}"; do
  [[ "$(cat "$file")" == 'existing edit' ]]
done

bash scripts/app.sh --update > passed.log
for file in Cargo.toml apps/example/Cargo.toml crates/example/Cargo.toml; do
  [[ "$(cat "$file")" == $'existing edit\nupgraded' ]]
done
[[ "$(cat Cargo.lock)" == $'existing edit\nresolved' ]]
