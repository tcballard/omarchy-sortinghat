# Development log

Append-only engineering handoff and limitations.

## 2026-08-31 — foundation

The repository began empty. The only default-branch change is an authorized one-line README bootstrap. Architecture, product contract, pinned toolchain and five compiling Rust crate boundaries were reconstructed on the feature branch.

Current limitations: the daemon protocol is only a truthful runtime skeleton; persistent roots/rules/proposals, active watching, recovery orchestration and QML are not yet wired. Filesystem primitives are separately tested, but the initial lexical helper is not an authorization boundary; the final service must use descriptor-relative `openat2` confinement described by ADR 0001.

## 2026-08-31 — persistent service and CLI

Added durable explicit roots, rules, proposals, pause state, bounded polling, stable-file sampling, partial-download filtering, verified MIME signatures, revision-aware review actions, owner-checked versioned IPC, no-replace moves, cross-filesystem staging/verification, conservative restart recovery and undo.

The service uses `openat2` confinement and identity revalidation before approval. The final rename itself still uses absolute-path `renameat2`; converting all mutation operands to retained parent descriptors remains a safety hardening gate. Polling is deliberate and bounded, but inotify overflow/debounce semantics are not yet represented. The sandbox blocks Unix-socket creation, so end-to-end daemon IPC remains unexecuted here.

## 2026-08-31 — Omarchy surface and packaging

Added the Quattro schemaVersion 1 service/bar-widget manifest, a bounded non-overlapping QML poller, review popup, folder settings, review actions, honest missing/paused/error states, keyboard focus targets, a no-IP-network user unit, reproducible committed-object packager, pinned upstream verification and a live capture gate. QML uses fixed argv arrays and plain-text rendering for untrusted values.

QML was statically validated but not loaded in a live Quickshell session. The host lacks a user systemd manager and rejects Unix sockets, so those two runtime integrations remain explicit acceptance gates.

## 2026-08-31 — exact-object and dependency assurance

The release archive was built from `git archive`, not the working tree, and then cleanly extracted and revalidated. Dependency policy now pins accepted registries, rejects wildcard requirements, scans RustSec advisories, and allows only the licences actually encountered on supported Linux targets. Generated archives remain untracked and no release was published.
