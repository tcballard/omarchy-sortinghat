# Machine-independent test matrix

| Required category | Evidence |
|---|---|
| Rule priority, identical outcomes, ties, filename patterns | `sortinghat-core` unit tests |
| MIME/extension disagreement | `mime_does_not_trust_extension` |
| Duplicate events | one-active-proposal guard in `only_explicit_roots_are_scanned…` |
| Partial downloads and growing files | service partial/growing tests |
| Same-filesystem move and undo | `approved_move_is_undoable_without_overwrite` |
| Cross-filesystem algorithm | descriptor-relative copy/publish/retire test; actual `EXDEV` mount unavailable in unprivileged CI |
| Exact and case-folding collision | fs no-replace and case-fold tests |
| Symlink, hardlink, traversal, hostile/non-UTF-8 names | fs/core/service hostile-path tests |
| Oversized files | sparse-file limit test |
| Interrupted/restart recovery | needs-attention and verified-source-removed restart tests |
| Agent unavailable/malformed/content denied | `sortinghat-agent` and service settings tests |
| Permission errors | errors are propagated and mapped to `filesystem_error`; deterministic permission injection is not yet implemented |
| Queue/journal limits | active queue saturation is fixture-tested; journal count, age and byte caps fail closed, but exhaustive 10,000-entry journal fixtures are not yet implemented |
| Live QML, systemd user manager, Unix IPC | live-only gate; this container rejects Unix sockets and has no user manager |

The two missing deterministic injection/limit fixtures above remain review findings, not passed claims. Live acceptance is tracked separately in `docs/live-acceptance.md`.
