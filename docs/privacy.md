# Privacy and agent boundary

SortingHat has no telemetry. Routine service output contains state and opaque identifiers; complete paths appear only in explicit local review responses because the user needs them to approve a move. File content is never logged.

Deterministic rules work offline and run first. The optional agent adapter is disabled by default, accepts bounded metadata, launches a configured local executable directly without a shell, and can only return a registered destination identifier or abstain. It cannot move files.

v0.1 exposes no content-reading or remote-agent setting. Consequently content cannot be sent to a model accidentally. A future content mode requires separate permissions for local read and remote transmission, bounded content, UI disclosure and new tests; metadata opt-in must not imply either permission.

