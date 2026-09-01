#!/usr/bin/env bash
set -euo pipefail

package_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
bin_dir="$HOME/.local/bin"
config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
unit_dir="$config_home/systemd/user"
install_root="$data_home/sortinghat"
provenance="$install_root/provenance"

if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
  printf 'SortingHat v0.1 release packages support x86_64 Linux only\n' >&2
  exit 1
fi

for required in \
  "$package_root/bin/sortinghat" \
  "$package_root/bin/sortinghatd" \
  "$package_root/systemd/sortinghat.service" \
  "$package_root/uninstall-user.sh" \
  "$package_root/SOURCE_COMMIT"; do
  if [[ ! -f $required ]]; then
    printf 'release package is incomplete: %s is missing\n' "$required" >&2
    exit 1
  fi
done

source_commit=$(<"$package_root/SOURCE_COMMIT")
if [[ ! $source_commit =~ ^[0-9a-f]{40}$ ]]; then
  printf 'release package has an invalid SOURCE_COMMIT\n' >&2
  exit 1
fi

read_provenance() {
  local key=$1
  [[ -f $provenance ]] || return 1
  sed -n "s/^${key}=//p" "$provenance" | head -n 1
}

verify_owned_or_absent() {
  local path=$1
  local key=$2
  [[ -e $path ]] || return 0
  local expected
  expected=$(read_provenance "$key") || {
    printf 'refusing to overwrite unowned path: %s\n' "$path" >&2
    return 1
  }
  local actual
  actual=$(sha256sum "$path" | awk '{print $1}')
  if [[ $actual != "$expected" ]]; then
    printf 'refusing to overwrite modified path: %s\n' "$path" >&2
    return 1
  fi
}

verify_owned_or_absent "$bin_dir/sortinghat" sortinghat_sha256
verify_owned_or_absent "$bin_dir/sortinghatd" sortinghatd_sha256
verify_owned_or_absent "$unit_dir/sortinghat.service" unit_sha256
verify_owned_or_absent "$install_root/uninstall-user.sh" uninstaller_sha256

if [[ -f $provenance ]]; then
  backup_root="$state_home/sortinghat-install-backup"
  mkdir -p "$backup_root"
  backup_dir=$(mktemp -d "$backup_root/$(date -u +%Y%m%dT%H%M%SZ).XXXXXX")
  mkdir -p "$backup_dir/bin" "$backup_dir/systemd" "$backup_dir/metadata"
  [[ ! -e $bin_dir/sortinghat ]] || cp -a "$bin_dir/sortinghat" "$backup_dir/bin/"
  [[ ! -e $bin_dir/sortinghatd ]] || cp -a "$bin_dir/sortinghatd" "$backup_dir/bin/"
  [[ ! -e $unit_dir/sortinghat.service ]] \
    || cp -a "$unit_dir/sortinghat.service" "$backup_dir/systemd/"
  cp -a "$provenance" "$backup_dir/metadata/"
fi

mkdir -p "$bin_dir" "$unit_dir" "$install_root"
install -m755 "$package_root/bin/sortinghat" "$bin_dir/sortinghat"
install -m755 "$package_root/bin/sortinghatd" "$bin_dir/sortinghatd"
install -m644 "$package_root/systemd/sortinghat.service" "$unit_dir/sortinghat.service"
install -m755 "$package_root/uninstall-user.sh" "$install_root/uninstall-user.sh"

provenance_next="$install_root/provenance.next"
{
  printf 'source_commit=%s\n' "$source_commit"
  printf 'sortinghat_sha256=%s\n' "$(sha256sum "$bin_dir/sortinghat" | awk '{print $1}')"
  printf 'sortinghatd_sha256=%s\n' "$(sha256sum "$bin_dir/sortinghatd" | awk '{print $1}')"
  printf 'unit_sha256=%s\n' "$(sha256sum "$unit_dir/sortinghat.service" | awk '{print $1}')"
  printf 'uninstaller_sha256=%s\n' \
    "$(sha256sum "$install_root/uninstall-user.sh" | awk '{print $1}')"
} > "$provenance_next"
chmod 600 "$provenance_next"
mv -f "$provenance_next" "$provenance"

systemctl --user daemon-reload
systemctl --user enable --now sortinghat.service

printf 'SortingHat runtime installed from commit %s\n' "$source_commit"
printf 'Uninstall with %s\n' "$install_root/uninstall-user.sh"
