#!/usr/bin/env bash
set -euo pipefail

bin_dir="$HOME/.local/bin"
config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
install_root="$data_home/sortinghat"
provenance="$install_root/provenance"
unit_path="$config_home/systemd/user/sortinghat.service"
backup_root="$state_home/sortinghat-uninstall-backup"

if [[ ! -f $provenance ]]; then
  printf 'refusing to remove files without SortingHat provenance: %s\n' "$provenance" >&2
  exit 1
fi

read_provenance() {
  local key=$1
  sed -n "s/^${key}=//p" "$provenance" | head -n 1
}

verify_owned_or_absent() {
  local path=$1
  local key=$2
  [[ -e $path ]] || return 0
  local expected
  expected=$(read_provenance "$key")
  local actual
  actual=$(sha256sum "$path" | awk '{print $1}')
  if [[ -z $expected || $actual != "$expected" ]]; then
    printf 'refusing to remove modified or unowned path: %s\n' "$path" >&2
    return 1
  fi
}

verify_owned_or_absent "$bin_dir/sortinghat" sortinghat_sha256
verify_owned_or_absent "$bin_dir/sortinghatd" sortinghatd_sha256
verify_owned_or_absent "$unit_path" unit_sha256
verify_owned_or_absent "$install_root/uninstall-user.sh" uninstaller_sha256

mkdir -p "$backup_root"
backup_dir=$(mktemp -d "$backup_root/$(date -u +%Y%m%dT%H%M%SZ).XXXXXX")
mkdir -p "$backup_dir/bin" "$backup_dir/systemd" "$backup_dir/metadata"

systemctl --user disable --now sortinghat.service 2>/dev/null || true
for binary in sortinghat sortinghatd; do
  source_path="$bin_dir/$binary"
  if [[ -e $source_path ]]; then
    mv --no-clobber "$source_path" "$backup_dir/bin/"
  fi
done
if [[ -e $unit_path ]]; then
  mv --no-clobber "$unit_path" "$backup_dir/systemd/"
fi
mv --no-clobber "$provenance" "$backup_dir/metadata/"
mv --no-clobber "$install_root/uninstall-user.sh" "$backup_dir/metadata/"
rmdir "$install_root" 2>/dev/null || true
systemctl --user daemon-reload 2>/dev/null || true

printf 'program files moved to %s\n' "$backup_dir"
printf 'state under %s was preserved; the Omarchy plugin was not modified\n' \
  "$state_home/sortinghat"
