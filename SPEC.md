# Omarchy-SortingHat v0.1

## Purpose

SortingHat is Omarchy's review-first file organisation layer. It watches only explicitly selected directories, applies deterministic typed rules first, optionally asks an explicitly permitted agent only after deterministic abstention, and queues explained proposals. It complements Nautilus; it is not an autonomous filing agent or document-management system.

## Authority

All implementation belongs here. Do not modify `tcballard/SortingHat`, OmaOffice, Annotated, OmaResearch, LocalWrap, or other Omarchy plugins. A feature branch and PR are authorised; merge, tag, release and marketplace submission are not.

## Product contract

1. Enrol watched and destination roots explicitly.
2. Detect a stable completed regular file.
3. Evaluate typed rules for extension, verified MIME, filename glob, source root/directory and completion evidence.
4. A unique top-priority outcome creates a proposal; conflicting top outcomes remain a disclosed tie.
5. An opted-in agent may supplement only no-match/tie outcomes. Metadata-only is the default. Content read and content transmission are independent permissions, both denied by default.
6. The panel explains source, destination, reason, provenance, warnings and undo limits.
7. Move, Rename, Choose folder, Create rule, Ignore and Undo are explicit actions. Rules and agents never move automatically.

## Hard safety invariants

- No watched-file mutation before durable revision-specific approval.
- No directory is watched without explicit selection.
- Retain enrolled root identity and resolve descendants FD-relative with Linux `openat2` beneath/no-symlink/no-magic-link/no-cross-mount flags.
- Accept only single-link regular files. Reject traversal, absolute paths, symlinks, hardlinks, directories, devices, sockets and FIFOs.
- Preserve raw Unix path bytes internally; expose bounded escaped display strings and opaque IDs.
- Treat discovery as a hint: bounded polling in v0.1 plus repeated stable inode/size/mtime/ctime samples, bounded MIME/hash inspection and revalidation at approval. A future inotify accelerator may not bypass settling.
- Debounce duplicates; overflow triggers bounded rescan and visible degraded/paused state.
- Never overwrite or silently rename a collision. Check exact and case-folding collisions; kernel no-replace is authoritative.
- Same-filesystem moves use `renameat2(RENAME_NOREPLACE)`, parent sync and destination verification.
- Cross-filesystem moves are disclosed before approval and use exclusive 0600 staging, bounded streaming copy, metadata policy, fsync, independent SHA-256 verification, no-replace publication, destination sync/reopen, source retirement, exact unlink, and source-directory sync.
- A post-approval `EXDEV` never silently changes the approved method.
- Journal states distinguish proposed, approved, copying, published, source_removed, completed, undoing, undone, failed_safely and needs_attention. `failed_safely` means a verified complete copy remains.
- Recovery trusts physical identity/checksum evidence, never path existence or the last journal state alone.
- Undo is journalled and refuses changed destinations or occupied originals.
- Queue/journal exhaustion pauses safely; active/nonterminal evidence is never pruned.
- Routine logs contain opaque IDs and coarse errors, never content or complete paths. No remote telemetry.

Cross-filesystem source unlink is permitted only as the final step of an explicitly approved, durably verified move; the product exposes no delete operation and never removes the last verified copy.

## Runtime architecture

- `sortinghat-core`: strict protocol, paths, rules and proposals.
- `sortinghat-journal`: single-writer SQLite WAL state, revisions, recovery and undo evidence.
- `sortinghat-fs`: enrolled roots, completion, MIME, movement and recovery primitives.
- `sortinghat-agent`: optional bounded metadata-first subprocess adapter.
- `sortinghat-cli`: `sortinghat` client and `sortinghatd` daemon over an owner-only Unix socket.
- root `manifest.json` plus thin `omarchy-plugin/` QML service/widget.
- hardened systemd user service with no core network access.

Every JSON boundary is schema version 1, bounded before parsing, deny-unknown, correlation-ID aware and revision checked. QML uses fixed argv arrays and opaque IDs—never `bash -c`.

## Required commands

`status`, `roots list/add/remove`, `queue list`, `proposal show/approve/rename/destination/ignore`, `rule create --proposal`, `undo`, `pause`, `resume`, and daemon `serve` must provide strict JSON output.

## Bounds

Hard defaults: 16 roots; depth 32; 25,000 watched directories; 1,000 rules; 1,000 active proposals; 4 GiB file; 64 KiB MIME prefix; 64 KiB IPC request; pages of 100; 2 KiB reason; 16 KiB agent metadata; separately permitted content 32 KiB; 30-second agent timeout; 10,000 terminal/30-day/64 MiB journal retention.

## Verification

Temporary-tree and injected-failure tests cover rule priority/ties, MIME disagreement, duplicates, partial/growing files, same/cross-FS moves, exact/case-fold collisions, symlinks/traversal, permissions, interrupted copy, crash before/after publication, restart recovery, undo, limits, hostile/non-UTF-8 names, oversized files, unavailable/malformed agent and denied content.

The exact committed object must pass format, Clippy, tests, dependency/licence/advisory and secret checks, strict manifest lint, the real pinned Omarchy validator, dynamic-text scan, clean archive extraction and revalidation.

Live Omarchy acceptance remains a separate captured gate for root selection, completed/partial download behavior, bar count, review actions, collision, rule creation, move, undo, restart recovery, pause, keyboard-only use and no unintended network traffic. Do not claim it without a real session.
