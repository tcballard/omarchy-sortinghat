#!/usr/bin/env bash
set -euo pipefail

backup_root=${XDG_STATE_HOME:-"$HOME/.local/state"}/sortinghat-uninstall-backup
backup_dir="$backup_root/$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$backup_dir/bin" "$backup_dir/systemd"

systemctl --user disable --now sortinghat.service 2>/dev/null || true
for binary in sortinghat sortinghatd; do
  source_path="$HOME/.local/bin/$binary"
  if [[ -e $source_path ]]; then
    mv --no-clobber "$source_path" "$backup_dir/bin/"
  fi
done
unit_path="$HOME/.config/systemd/user/sortinghat.service"
if [[ -e $unit_path ]]; then
  mv --no-clobber "$unit_path" "$backup_dir/systemd/"
fi
systemctl --user daemon-reload 2>/dev/null || true

printf 'program files moved to %s\n' "$backup_dir"
printf 'state under %s was preserved; the Omarchy plugin was not modified\n' \
  "${XDG_STATE_HOME:-"$HOME/.local/state"}/sortinghat"
