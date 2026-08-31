# Pinned Omarchy Quattro contract

At task start on 2026-08-30, `omacom/omarchy` `refs/heads/quattro` resolved to `981274b20af8e85c09845071ac33c6230909f119`. The branch moved later; this start-time pin remains authoritative.

Authoritative paths at that commit:

- `manual/32-shell-plugins.md`
- `docs/omarchy-shell.md`
- `shell/README.md`
- `agents/skills/shell-dev.md`
- `shell/services/PluginRegistry.qml`
- `shell/shell.qml`
- `bin/omarchy-shell`
- `bin/omarchy-plugin-validate`
- `test/shell.d/plugin-validate-test.sh`
- `test/shell.d/qml-text-format-test.sh`
- `test/shell.d/qml-text-format-scan.py`

Validator blob: `00d751229d1c927aef8ca0c3843692984a254789`  
Validator SHA-256: `f7507e5042eb970e3dc918bdf6bf251c7557443892a77e71a17d7019ddde72c8`

There is no separate JSON Schema. Runtime loading plus the validator define the contract. The root `manifest.json` uses numeric `schemaVersion: 1`, supported unique kinds only, valid relative regular-file entry points and no symlinks outside `.git`.

SortingHat uses `service` + `bar-widget`. The service owns a bounded non-overlapping status poll; the widget reads it through `bar.shell.serviceFor(moduleName)` and provides native panel/keyboard behavior. Dynamic strings use `Text.PlainText`. Commands are argv vectors, never shell strings.

`omarchy plugin add` does not install binaries or systemd units, so plugin-only installation must truthfully show `runtime missing`.

Validation runs against an exact `git archive` extraction using the validator at the pinned commit, plus strict project lint and the upstream dynamic-text scanner. Live Wayland loader/visual/keyboard behavior is not implied by static validation.
