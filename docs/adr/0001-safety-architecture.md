# ADR 0001: Review-first Linux trust boundary

- Status: accepted for v0.1 implementation
- Date: 2026-08-30

## Decision

One Rust daemon is the sole owner of watched-file mutations and the SQLite journal. The CLI communicates over a mode-0600 Unix socket with peer-UID verification. QML is presentation only.

Roots are explicit capabilities: retain `O_PATH|O_DIRECTORY` descriptors and identities, resolve descendants with `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_MAGICLINKS|RESOLVE_NO_XDEV)`, and perform mutations through parent FDs plus raw-byte basenames. `canonicalize()`/prefix checks are display validation only.

Inotify starts settling but cannot prove completion. Stable repeated identity/size/time observations, bounded FD-based MIME/hash inspection, and approval-time revalidation bind every proposal.

Rules are typed and pure. All highest-priority matches are considered; one normalised outcome decides, different outcomes tie, and no match abstains. Agent classification is disabled by default, metadata-first, single-flight, direct-argv, bounded/cancellable, and returns a registered destination ID or abstains.

SQLite uses WAL and `synchronous=FULL`. Durable intent precedes mutation; file and directory durability precede success. Same-FS movement uses `renameat2` no-replace. Cross-FS movement stages, verifies, publishes, reopens, then retires and removes the exact source inode. Recovery verifies identities/checksums. Ambiguity preserves copies and enters `needs_attention`.

Undo repeats the same confined no-overwrite protocol. Queue/journal caps pause mutation. The core service runs with no network; optional agent execution cannot weaken filesystem authority.

## Consequences

Linux `openat2`, `statx`, `renameat2`, inotify and SQLite are baseline requirements. Unsupported kernels/filesystems fail closed. Descriptor-relative raw-byte handling is more complex, but prevents symlink, mount and path-replacement races that the macOS reference does not address.

## Rejected

- security-sensitive QML or shell commands;
- `canonicalize()` plus lexical prefix as authority;
- `exists` then `rename` collision handling;
- automatic numbered destinations;
- implicit post-approval same/cross-FS fallback;
- tolerant prose-wrapped agent JSON;
- whole-file memory reads;
- moving-main plugin scaffolding (no tagged release exists).
