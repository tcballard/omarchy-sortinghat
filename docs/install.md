# Install and operate v0.1

## Runtime

Build from a reviewed commit and install the two binaries plus the user unit:

```bash
cargo build --release --locked
install -Dm755 target/release/sortinghat "$HOME/.local/bin/sortinghat"
install -Dm755 target/release/sortinghatd "$HOME/.local/bin/sortinghatd"
install -Dm644 packaging/systemd/sortinghat.service "$HOME/.config/systemd/user/sortinghat.service"
systemctl --user daemon-reload
systemctl --user enable --now sortinghat.service
```

The unit has no IP networking capability. State is private under `~/.local/state/sortinghat`; the owner-only socket is `%t/sortinghat.sock`.

## Omarchy plugin

Add this repository using Omarchy's normal plugin flow and enable `tcballard.sortinghat`. Omarchy only installs QML, not the Rust runtime. Until the runtime is installed and running, the widget deliberately reports `runtime missing`.

Select the first watched folder from the panel's **Watch folder** button. The same explicit selection is registered as an allowed destination root. No default folder is watched.

## CLI

Every response is schema-versioned JSON. Examples:

```bash
sortinghat --json status
sortinghat --json roots list
sortinghat --json roots add "$HOME/Downloads" --watch --destination
sortinghat --json queue list
sortinghat --json proposal show PROPOSAL_ID
sortinghat --json proposal choose-folder PROPOSAL_ID "$HOME/Documents" --revision REVISION
sortinghat --json proposal rename PROPOSAL_ID new-name.pdf --revision REVISION
sortinghat --json proposal approve PROPOSAL_ID --revision REVISION
sortinghat --json rule create --proposal PROPOSAL_ID --priority 100
sortinghat --json proposal ignore PROPOSAL_ID --revision REVISION
sortinghat --json undo PROPOSAL_ID --revision REVISION
sortinghat --json pause
sortinghat --json resume
```

Optional agent classification remains off until explicitly enabled. It receives only bounded metadata and cannot move a file:

```bash
sortinghat --json agent status
sortinghat --json agent enable /absolute/path/to/local-adapter --arg fixed-argument
sortinghat --json agent disable
```

There is no content permission in v0.1; enabling metadata classification does not grant one.

Re-read a proposal after any action: successful edits increment its revision and stale approvals are rejected.

## Uninstall

`./scripts/uninstall-user.sh` disables the unit and moves installed program files into a timestamped recovery folder. It does not delete SortingHat state, watched files or the Omarchy plugin. Remove the plugin separately through Omarchy after review.
