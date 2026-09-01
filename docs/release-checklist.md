# v0.1.0 release and marketplace checklist

## Verified on the candidate branch

- [x] Plugin ID is the permanent namespaced `io.github.tcballard.sortinghat`.
- [x] Root manifest, README, Apache-2.0 licence and dependency documentation are present.
- [x] Root README contains release-package installation and recoverable removal instructions.
- [x] Prebuilt x86_64 runtime packaging is reproducible from an exact committed object.
- [x] Installer and uninstaller verify ownership and refuse modified or unowned files.
- [x] Rust tests, Clippy, formatting, dependency policy, installer lifecycle, systemd analysis, pinned Omarchy validation, QML dynamic-text scanning and secret-pattern scanning pass.
- [x] CI runs for pull requests, the feature branch and `main`.
- [x] Marketplace ID search found no collision before the candidate change.

## Live and publication gates

- [ ] Run `scripts/capture-live-acceptance.sh` on current Omarchy Quattro and complete every observation with synthetic fixtures.
- [ ] Attach a genuine, privacy-safe product screenshot as root `preview.png`.
- [ ] Review and merge PR #9; verify post-merge CI on the exact `main` commit.
- [ ] Build `sortinghat-0.1.0-x86_64-unknown-linux-gnu.tar.gz` and its checksum from that exact commit.
- [ ] Publish `v0.1.0` and attach both runtime assets; verify the README download path end to end.
- [ ] Replace “release candidate” with the release date/status without changing product claims.
- [ ] Confirm all five marketplace ownership and safety statements in `docs/marketplace-submission.md`.
- [ ] Submit the reviewed marketplace issue and respond to exact-commit compatibility/security reports.
