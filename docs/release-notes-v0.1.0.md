# SortingHat v0.1.0

The first public release of SortingHat: review-first file organisation for Omarchy Quattro.

## What ships

- Explicit watched and destination folders—SortingHat assumes no default roots.
- Deterministic typed rules with explained proposals and an ambiguity review queue.
- Approve, rename, choose-folder, create-rule, ignore and undo actions.
- No-overwrite same-filesystem moves and verified staged cross-filesystem publication.
- Durable recovery evidence and conservative restart handling.
- An optional metadata-only local-agent boundary, disabled by default.
- A native Omarchy bar widget and review panel backed by a separate Rust service.

## Safety and privacy

Files never move without approval, collisions never overwrite, and v0.1 has no file-content permission or telemetry. Deterministic organisation works offline. The installer records exact file provenance and refuses modified or unowned runtime targets.

## Availability

The attached prebuilt runtime targets x86_64 Omarchy Quattro systems with a user systemd manager. Installation and recoverable removal instructions are in the [README](https://github.com/tcballard/omarchy-sortinghat#install).

The complete machine-independent verification suite passes, and the packaged install was owner-verified on Omarchy Quattro before publication.
