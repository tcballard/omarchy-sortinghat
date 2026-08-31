# Live Omarchy acceptance gate

Status: **not run**. A real current Omarchy Quattro Wayland session is required.

Start a capture with:

```bash
./scripts/capture-live-acceptance.sh live-acceptance.md
```

The harness records the exact commit, Omarchy version, unit state and a checklist. The tester must use a dedicated disposable folder and attach screenshots or a screen recording for UI claims.

Required observations:

- select a watched directory; confirm no other directory appears;
- create a completed download and a `.crdownload` that is still changing;
- confirm the bar count and each review field;
- exercise choose folder, rename, collision, create rule, move, ignore and undo;
- interrupt the service during a staged fixture and confirm conservative recovery;
- pause and resume;
- complete the panel using keyboard only;
- capture socket/process traffic and confirm no unintended IP networking.

Static validation, unit tests and a mocked compositor do not satisfy this gate.

