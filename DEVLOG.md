# Development log

Append-only engineering handoff and limitations.

## 2026-08-31 — foundation

The repository began empty. The only default-branch change is an authorized one-line README bootstrap. Architecture, product contract, pinned toolchain and five compiling Rust crate boundaries were reconstructed on the feature branch.

Current limitations: the daemon protocol is only a truthful runtime skeleton; persistent roots/rules/proposals, active watching, recovery orchestration and QML are not yet wired. Filesystem primitives are separately tested, but the initial lexical helper is not an authorization boundary; the final service must use descriptor-relative `openat2` confinement described by ADR 0001.

