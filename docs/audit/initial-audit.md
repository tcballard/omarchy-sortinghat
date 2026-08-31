# Initial audit

- Implementation repository was empty; issues #1–#8 are the delivery contracts.
- Read-only reference: `tcballard/SortingHat` commit `6b340d80a3c388722735f8691b01af2a568708f9`, tree `8c7876fa9c61d65a27ecd65bac4d9ec27f639763`, Apache-2.0.
- Retained ideas: separate roots, abstention/review, stable rules, shared preview/validation path, separate rule-learning approval, SHA-256 staging and fail-closed recovery.
- Rejected hazards: model-first auto-apply, no completion detection, lexical path checks, unjournalled moves, identity-blind undo, implicit collision suffixes, unbounded agent waits/output, automatic content transmission, whole-file reads and Apple-only assumptions.
- `tcballard/build-omarchy-plugins` had no tags or releases, so moving `main` is not used. Scaffolding is directly against pinned official Quattro.
- Live Omarchy acceptance is unavailable in this container and remains explicit.
