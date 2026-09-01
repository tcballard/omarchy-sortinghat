# Build log

Append-only verification evidence for `feat/machine-independent-v0.1`.

## 2026-08-31 — foundation

- Base: `main` at `719302b75952e05bca5948432b45aa47ffc957a1` (README-only bootstrap).
- Target: Omarchy Quattro `981274b20af8e85c09845071ac33c6230909f119`.
- Validator: upstream blob `00d751229d1c927aef8ca0c3843692984a254789`, SHA-256 `f7507e5042eb970e3dc918bdf6bf251c7557443892a77e71a17d7019ddde72c8`.
- `tcballard/build-omarchy-plugins`: no tagged release exists; moving `main` was not inspected.
- `cargo test --workspace`: passed, 10 unit tests plus doc tests.

## 2026-08-31 — persistent service and CLI

- `cargo test --workspace`: passed, 15 unit tests plus doc tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- Daemon CLI smoke test: blocked. This execution sandbox returned `EPERM` while creating the Unix socket; no live daemon claim is made.

## 2026-08-31 — Omarchy surface and packaging

- Pinned upstream `omarchy-plugin-validate`: passed; verified validator SHA-256 before execution.
- Pinned upstream `qml-text-format-scan.py`: passed.
- `cargo test --workspace --locked`: passed, 15 unit tests plus doc tests.
- `systemd-analyze --user verify`: blocked because this container has no user manager/runtime directory. System-level parse reached only the expected missing uninstalled binary check.
- Live Omarchy UI/keyboard acceptance: not run; capture harness added and gate remains explicit.

## 2026-08-31 — exact-object and dependency assurance

- Source object: `2c1b56a22c403a14033cb93251a2a31e82094c7a`.
- Reproducible release archive SHA-256: `73ad5a4f8265d1f6032c04af5c2040e9fbcc63180788399c670383d912966a48`.
- Clean extraction `SOURCE_COMMIT`: matched the source object.
- Extracted binaries: `sortinghat-cli 0.1.0`; daemon help executed.
- Extracted plugin: official pinned validator and dynamic-text scan passed.
- `cargo-deny 0.20.2 check`: advisories, bans, licences and sources passed with an explicit Linux-target policy.
- `scripts/verify.sh`: passed end to end, including format, Clippy, tests, locked metadata, dependency policy, official validator, QML scan and bounded secret-pattern scan.

## 2026-08-31 — descriptor-relative mutation and agent opt-in

- `cargo test --workspace --locked`: passed, 21 unit tests plus doc tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Added descriptor-relative same-filesystem rename, staged publication, verified source retirement and undo tests.
- Added restart interruption, hardlink, oversized sparse file and case-folding coverage.
- Agent is disabled by default, metadata-only when explicitly configured, 30-second bounded, destination-constrained and incapable of content access or mutation.

## 2026-08-31 — evidence-based restart recovery

- `cargo test --workspace --locked`: passed, 22 unit tests plus doc tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Restart after `source_removed` advances to `completed` only when the source is absent and the destination matches recorded identity/size/checksum evidence.
- All other interrupted mutation states preserve copies and enter `needs_attention` with a coarse evidence summary.

## 2026-08-31 — bounded protocol and retention gates

- `scripts/verify.sh`: passed end to end with 28 unit tests plus doc tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo-deny 0.20.2 check`: advisories, bans, licences and sources passed.
- Pinned upstream manifest validator and dynamic QML text scan: passed.
- Offline systemd security/parse analysis: passed; no live user manager claim is made.
- Added strict request validation, queue/walk/journal bounds, terminal retention, recoverable uninstall, supported cross-filesystem mode/time preservation and duplicate-identity suppression.

## 2026-09-01 — deterministic release-gate fixtures

- `cargo test --workspace --locked`: passed, 31 unit tests plus doc tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Injected `EACCES` at the descriptor-relative no-replace rename boundary; the source remained byte-for-byte intact, no destination appeared and the daemon mapped the propagated failure to `filesystem_error`.
- Seeded a real SQLite journal with 10,001 terminal entries plus one active proposal; retention removed exactly one oldest terminal row, retained 10,000 terminal rows and preserved active evidence.

## 2026-09-01 — marketplace release preparation

- `scripts/verify.sh`: passed end to end with 31 unit tests plus doc tests.
- Formatting, Clippy with warnings denied, locked metadata, `cargo-deny`, installer lifecycle, offline systemd analysis, the pinned official validator, QML dynamic-text scan and bounded secret-pattern scan all passed.
- Changed the permanent candidate ID to `io.github.tcballard.sortinghat`; no marketplace collision was found before the change.
- Added versioned x86_64 release packaging with a local installer, exact source provenance, owned-upgrade recovery and modified/unowned-file refusal. Rust is no longer a release-install dependency.
- Root README now directly documents requirements, checksum verification, installation and recoverable removal. Release and marketplace drafts preserve live-only claims as blocked.
