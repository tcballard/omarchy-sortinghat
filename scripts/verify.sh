#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cargo_bin=${CARGO:-cargo}
quattro_sha=981274b20af8e85c09845071ac33c6230909f119
validator_sha256=f7507e5042eb970e3dc918bdf6bf251c7557443892a77e71a17d7019ddde72c8
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

cd "$repo_root"
"$cargo_bin" fmt --all -- --check
"$cargo_bin" clippy --workspace --all-targets --locked -- -D warnings
"$cargo_bin" test --workspace --locked
"$cargo_bin" metadata --locked --format-version 1 >/dev/null
cargo-deny check
systemd-analyze security --offline=yes packaging/systemd/sortinghat.service >/dev/null

curl --fail --silent --show-error --location \
  "https://raw.githubusercontent.com/omacom/omarchy/$quattro_sha/bin/omarchy-plugin-validate" \
  --output "$work_dir/omarchy-plugin-validate"
printf '%s  %s\n' "$validator_sha256" "$work_dir/omarchy-plugin-validate" | sha256sum --check --status
chmod 700 "$work_dir/omarchy-plugin-validate"
"$work_dir/omarchy-plugin-validate" "$repo_root"

curl --fail --silent --show-error --location \
  "https://raw.githubusercontent.com/omacom/omarchy/$quattro_sha/test/shell.d/qml-text-format-scan.py" \
  --output "$work_dir/qml-text-format-scan.py"
mkdir -p "$work_dir/scan/shell"
cp "$repo_root"/omarchy-plugin/*.qml "$work_dir/scan/shell/"
python3 "$work_dir/qml-text-format-scan.py" "$work_dir/scan"

if rg -n --hidden --glob '!Cargo.lock' --glob '!.git/**' \
  '(BEGIN (RSA|OPENSSH|EC) PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16})' .; then
  printf 'potential secret detected\n' >&2
  exit 1
fi
