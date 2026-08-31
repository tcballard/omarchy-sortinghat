# Repository contract

`SPEC.md` is authoritative. Verified tests outrank logs; explicit later user decisions outrank earlier prose.

- Modify only this repository. `tcballard/SortingHat` and all other named products are read-only evidence.
- Work issue-first on `feat/machine-independent-v0.1`; never merge, tag, release, publish, or submit to the marketplace.
- Keep QML thin. Filesystem, classification, agent execution, moves, journal, recovery and undo belong in Rust.
- Never interpolate paths or filenames into shell commands.
- Never mutate a watched file before durable revision-specific approval.
- Use logical commits and push every completed slice before beginning the next.
- Record exact verification in `BUILDLOG.md` and candid limitations in `DEVLOG.md`.
