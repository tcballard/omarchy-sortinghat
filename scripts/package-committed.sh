#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
commit=${1:-HEAD}
commit_sha=$(git -C "$repo_root" rev-parse "$commit^{commit}")
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
source_dir="$work_dir/source"
package_dir="$work_dir/sortinghat-$commit_sha"
mkdir -p "$source_dir" "$package_dir/bin" "$package_dir/plugin" "$package_dir/systemd"

git -C "$repo_root" archive "$commit_sha" | tar -x -C "$source_dir"
cd "$source_dir"
"${CARGO:-cargo}" build --release --locked --workspace
install -m755 target/release/sortinghat target/release/sortinghatd "$package_dir/bin/"
cp manifest.json "$package_dir/plugin/"
cp -R omarchy-plugin "$package_dir/plugin/"
install -m644 packaging/systemd/sortinghat.service "$package_dir/systemd/"
install -m644 README.md LICENSE docs/install.md docs/privacy.md "$package_dir/"
printf '%s\n' "$commit_sha" > "$package_dir/SOURCE_COMMIT"

mkdir -p "$repo_root/dist"
: "${SOURCE_DATE_EPOCH:=$(git -C "$repo_root" show -s --format=%ct "$commit_sha")}" 
tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
  -czf "$repo_root/dist/sortinghat-$commit_sha.tar.gz" -C "$work_dir" "sortinghat-$commit_sha"
sha256sum "$repo_root/dist/sortinghat-$commit_sha.tar.gz" > "$repo_root/dist/sortinghat-$commit_sha.tar.gz.sha256"

