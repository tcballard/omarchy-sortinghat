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
