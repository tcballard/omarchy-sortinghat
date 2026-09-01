#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
commit=${1:-HEAD}
commit_sha=$(git -C "$repo_root" rev-parse "$commit^{commit}")
version=${2:-$(git -C "$repo_root" show "$commit_sha:manifest.json" | sed -n 's/^[[:space:]]*"version": "\([^"]*\)",/\1/p')}
target=x86_64-unknown-linux-gnu
if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'invalid release version: %s\n' "$version" >&2
  exit 1
fi
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
source_dir="$work_dir/source"
package_name="sortinghat-$version-$target"
package_dir="$work_dir/$package_name"
mkdir -p "$source_dir" "$package_dir/bin" "$package_dir/plugin" "$package_dir/systemd"

git -C "$repo_root" archive "$commit_sha" | tar -x -C "$source_dir"
cd "$source_dir"
"${CARGO:-cargo}" build --release --locked --workspace
install -m755 target/release/sortinghat target/release/sortinghatd "$package_dir/bin/"
cp manifest.json "$package_dir/plugin/"
cp -R omarchy-plugin "$package_dir/plugin/"
install -m644 packaging/systemd/sortinghat.service "$package_dir/systemd/"
install -m755 scripts/install-package.sh "$package_dir/install.sh"
install -m755 scripts/uninstall-user.sh "$package_dir/uninstall-user.sh"
install -m644 README.md LICENSE docs/install.md docs/privacy.md "$package_dir/"
printf '%s\n' "$commit_sha" > "$package_dir/SOURCE_COMMIT"

mkdir -p "$repo_root/dist"
: "${SOURCE_DATE_EPOCH:=$(git -C "$repo_root" show -s --format=%ct "$commit_sha")}" 
tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
  -czf "$repo_root/dist/$package_name.tar.gz" -C "$work_dir" "$package_name"
(
  cd "$repo_root/dist"
  sha256sum "$package_name.tar.gz" > "$package_name.tar.gz.sha256"
)
