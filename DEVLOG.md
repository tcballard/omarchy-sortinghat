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

## 2026-08-31 — descriptor-relative mutation and agent opt-in

Closed the earlier absolute-mutation gap: approval now opens confined source and destination parents and supplies those retained descriptors plus raw basenames to `renameat2`, staging, publication, verification, exact unlink and undo. Cross-filesystem retirement reopens and verifies the exact source through the retained parent immediately before unlink.

Wired the optional adapter after deterministic tie/abstention only. Enabling it is an explicit CLI setting; requests contain extension, verified MIME, coarse size bucket, source-root ID and allowed destination IDs. v0.1 has no content grant. Adapter failure or malformed output leaves an ordinary review proposal.

The watcher ADR now matches implementation: bounded polling is authoritative for v0.1; a future inotify accelerator cannot bypass the same stable-sampling and bounded-rescan path.

## 2026-08-31 — evidence-based restart recovery

Recovery now distinguishes a provably finished move from ambiguous interruption. It may complete only a `source_removed` record whose source is absent and whose destination matches same-filesystem inode evidence or the recorded cross-filesystem SHA-256. Approved/copying/published/undoing records become `needs_attention`; warnings disclose whether a verified source, verified destination, both, or neither are present. No recovery path retries mutation automatically.

## 2026-08-31 — bounded protocol and retention gates

The daemon now rejects unknown fields, non-v1 requests, nil correlation IDs and oversized arguments before dispatch. Active queue saturation pauses the watcher rather than dropping work. Traversal, state, journal age/count/bytes and rule/agent inputs have explicit caps; retention deletes terminal records only.

Same-filesystem moves sync and reverify both directory sides. Staged cross-filesystem copies apply the source mode and access/modification times before durable publication. An ignored or already-seen physical identity does not immediately re-enter review. The uninstall helper is recoverable and preserves plugin data and state.

The machine-independent test matrix remains candid: deterministic permission-failure injection and exhaustive 10,000-entry journal fixtures are not claimed. Live Quickshell, user-systemd and Unix-socket acceptance remain gated by the unavailable host capabilities.

## 2026-09-01 — marketplace release preparation

The two deterministic test gaps are now closed. Permission denial is injected at the descriptor-relative rename boundary and maps to the public filesystem-error state without mutation. A real 10,001-terminal-row SQLite fixture proves the 10,000-entry retention limit while preserving active evidence.

The release path no longer requires users to compile Rust. Exact-commit packaging produces a versioned x86_64 archive, checksum, provenance-aware installer and recoverable uninstaller. Installation refuses existing paths unless their hashes match SortingHat's prior provenance; removal likewise refuses modified or unowned files.

Live Quickshell rendering, keyboard-only use, user-systemd activation, daemon/CLI IPC, genuine preview capture and unintended-network observation still require a real Omarchy Quattro session. No release or marketplace claim should cross that boundary until the capture is completed.
