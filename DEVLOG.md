# Development log

Append-only engineering handoff and limitations.

## 2026-08-31 — foundation

The repository began empty. The only default-branch change is an authorized one-line README bootstrap. Architecture, product contract, pinned toolchain and five compiling Rust crate boundaries were reconstructed on the feature branch.

Current limitations: the daemon protocol is only a truthful runtime skeleton; persistent roots/rules/proposals, active watching, recovery orchestration and QML are not yet wired. Filesystem primitives are separately tested, but the initial lexical helper is not an authorization boundary; the final service must use descriptor-relative `openat2` confinement described by ADR 0001.

## 2026-08-31 — persistent service and CLI

Added durable explicit roots, rules, proposals, pause state, bounded polling, stable-file sampling, partial-download filtering, verified MIME signatures, revision-aware review actions, owner-checked versioned IPC, no-replace moves, cross-filesystem staging/verification, conservative restart recovery and undo.

The service uses `openat2` confinement and identity revalidation before approval. The final rename itself still uses absolute-path `renameat2`; converting all mutation operands to retained parent descriptors remains a safety hardening gate. Polling is deliberate and bounded, but inotify overflow/debounce semantics are not yet represented. The sandbox blocks Unix-socket creation, so end-to-end daemon IPC remains unexecuted here.
