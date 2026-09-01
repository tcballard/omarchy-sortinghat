# SortingHat for Omarchy

Review-first file organisation for Omarchy Quattro.

SortingHat watches only folders you explicitly select. Deterministic rules propose where completed files should go; ambiguous files remain in a review queue and may use an optional metadata-only agent after opt-in. Nothing moves until you approve it, destinations are explained, collisions never overwrite, and completed moves are journalled for recovery and undo.

The native Rust service does the filesystem work. The Omarchy QML service and bar widget are deliberately thin. SortingHat complements the existing file manager; it does not replace Nautilus or become a general document-management system.

## Safety and privacy

- No folder is watched until you select it.
- Rules and agents create proposals; they cannot move files automatically.
- Moves never overwrite or silently merge directories.
- Interrupted operations recover from recorded filesystem evidence instead of guessing.
- Deterministic organisation works offline.
- The optional agent is disabled by default and receives bounded metadata only.
- v0.1 has no file-content permission, telemetry or core IP networking.

See [the privacy boundary](docs/privacy.md) and [the architecture decision](docs/adr/0001-safety-architecture.md) for the full contract.

## Status

Version 0.1.0 is a release candidate. Machine-independent verification passes; live Omarchy UI, keyboard, service-manager and network-observation acceptance remains a separate required gate and is not yet claimed.

## Install

Requirements: Omarchy Quattro on x86_64 Linux, a user systemd manager, and `curl`, `tar` and `sha256sum`. The release runtime is prebuilt; Rust is not required.

Install the reviewed v0.1.0 runtime package after the release is published:

```bash
version=0.1.0
asset="sortinghat-${version}-x86_64-unknown-linux-gnu.tar.gz"
base="https://github.com/tcballard/omarchy-sortinghat/releases/download/v${version}"
work_dir=$(mktemp -d)
curl --fail --location --output "$work_dir/$asset" "$base/$asset"
curl --fail --location --output "$work_dir/$asset.sha256" "$base/$asset.sha256"
(cd "$work_dir" && sha256sum --check "$asset.sha256")
tar -xzf "$work_dir/$asset" -C "$work_dir"
"$work_dir/${asset%.tar.gz}/install.sh"
```

The installer records exact file hashes, refuses to overwrite an unowned or modified runtime, and retains a recovery copy during an owned upgrade.

Then add and enable the plugin through Omarchy:

```bash
omarchy plugin add https://github.com/tcballard/omarchy-sortinghat
```

Enable `io.github.tcballard.sortinghat` and add its bar widget. Until the runtime is installed and healthy, the widget deliberately reports `runtime missing`.

Open the panel and choose **Watch folder** to enrol the first watched and destination root. SortingHat never assumes `~/Downloads` or any other default folder.

For source builds and CLI operation, see [docs/install.md](docs/install.md).

## Remove

Remove the runtime with:

```bash
"${XDG_DATA_HOME:-$HOME/.local/share}/sortinghat/uninstall-user.sh"
```

The uninstaller verifies ownership, disables the user service and moves installed program files into a timestamped recovery folder. It preserves SortingHat state and every watched file. Review that backup, then remove the plugin separately through Omarchy.

## Verification

```bash
./scripts/verify.sh
```

The gate includes formatting, Clippy with warnings denied, the complete Rust test suite, dependency/advisory/licence policy, installer ownership tests, the pinned upstream Omarchy validator, the upstream dynamic-text scan, systemd hardening analysis and a bounded secret-pattern scan.

Live acceptance is intentionally separate and documented in [docs/live-acceptance.md](docs/live-acceptance.md).

## License

Apache-2.0. External Rust dependencies and their accepted licences are locked and checked by `cargo-deny`.
