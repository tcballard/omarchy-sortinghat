# Marketplace submission draft

Do not submit this draft until `docs/release-checklist.md` is complete and the owner has reviewed every marketplace checkbox.

Title: `[Plugin]: SortingHat`

```markdown
### Repository URL

https://github.com/tcballard/omarchy-sortinghat

### Category

Productivity

### Tags

bar, quickshell, system

### Suggest a missing tag

files

### Maintainer notes

SortingHat is a review-first file organisation service and bar widget. It ships a separate checksum-verified x86_64 runtime package and a hardened systemd user service. The installer and uninstaller verify file provenance and refuse modified or unowned targets. Deterministic operation is offline; the optional local agent is disabled by default and receives bounded metadata only. v0.1 has no file-content permission or telemetry.

The Automated Security Baseline is expected to report installer and service-management capabilities requiring maintainer review. No passwordless privilege policy, privileged helper or bundled executable is present in the plugin repository.

### Submission checklist

- [ ] The repository is public and contains installation and removal instructions.
- [ ] I have documented the plugin license and any external dependencies.
- [ ] I confirm that I own or have permission to submit this plugin and its preview assets.
- [ ] The plugin does not overwrite user configuration without explicit consent.
- [ ] I understand that approval is for listing and is not a security review.
```
