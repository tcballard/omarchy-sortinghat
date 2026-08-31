#!/usr/bin/env bash
set -euo pipefail

output=${1:-live-acceptance.md}
test ! -e "$output" || { printf 'refusing to overwrite %s\n' "$output" >&2; exit 1; }
commit=$(git rev-parse HEAD)
omarchy_version=$(omarchy --version 2>&1 || true)
unit_state=$(systemctl --user is-active sortinghat.service 2>&1 || true)

sed "s/@COMMIT@/$commit/; s/@OMARCHY@/$omarchy_version/; s/@UNIT@/$unit_state/" > "$output" <<'REPORT'
# SortingHat live acceptance capture

- Commit: `@COMMIT@`
- Omarchy: `@OMARCHY@`
- Unit at start: `@UNIT@`
- Date/time:
- Tester:
- Dedicated fixture root:

## Evidence checklist

- [ ] Explicit watched-directory selection
- [ ] Completed download queued
- [ ] Partial/growing download ignored
- [ ] Bar count updated
- [ ] Source, destination, reason, provenance and warning visible
- [ ] Choose folder and rename
- [ ] Collision refused without overwrite
- [ ] Rule created and applied deterministically
- [ ] Approved move
- [ ] Undo
- [ ] Restart recovery
- [ ] Pause and resume
- [ ] Keyboard-only operation
- [ ] No unintended network traffic

## Attachments and notes

Do not paste personal paths, filenames or content. Use synthetic fixtures and redact captures.
REPORT
printf 'capture created at %s; complete it in a live Omarchy session\n' "$output"

