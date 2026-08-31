# Omarchy-SortingHat

An intelligent, review-first file organisation layer for Omarchy Quattro.

SortingHat watches only folders you explicitly select. Deterministic rules propose where completed files should go; ambiguous files stay in a review queue and may use an optional, metadata-first agent only after opt-in. Nothing moves until you approve it, destinations are explained, collisions never overwrite, and completed moves are journalled for recovery and undo.

This is a native Rust service with a thin Omarchy QML surface. It complements the existing file manager—it does not replace Nautilus or become a general document-management system.

## Status

v0.1 is under active development on `feat/machine-independent-v0.1`. Live Omarchy acceptance is a required, separate gate and has not been claimed.

See [SPEC.md](SPEC.md), [ADR 0001](docs/adr/0001-safety-architecture.md), and the [initial audit](docs/audit/initial-audit.md).
