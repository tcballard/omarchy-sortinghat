<h1 align="center">SortingHat for Omarchy</h1>

<p align="center">
  <a href="https://github.com/tcballard/omarchy-badges"><img src="https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-plugin.svg" alt="Built for Omarchy: Plugin" height="24"></a>
</p>

**Put files in their place, with the final say.**

SortingHat watches folders you choose and proposes where completed files should go. Review the destination and reason, approve a move, or leave the file where it is. Nothing moves automatically.

## Everyday use

Open the panel and choose **Watch folder**. Rules suggest destinations for completed files; ambiguous items stay in the review queue. Approved moves are recorded for recovery and undo, and collisions never overwrite an existing file.

## Install

Omarchy Quattro on x86_64 Linux, a systemd user session, curl, tar and sha256sum. The release runtime is prebuilt; Rust is not required.

Install the checksum-verified runtime using the [v0.1.0 installation steps](GUIDE.md#install), then add the plugin:

```bash
omarchy plugin add https://github.com/tcballard/omarchy-sortinghat
```

Enable **SortingHat** and add its bar widget. The native runtime is required before the plugin can work.

## Update and remove

This plugin also has a native runtime. Follow the [runtime and plugin instructions](GUIDE.md#remove) when updating or removing it. The shell plugin can be removed with `omarchy plugin remove io.github.tcballard.sortinghat`.

## A few useful details

**v0.1.0.** The packaged install was owner-verified on Omarchy Quattro. Detailed live acceptance is recorded separately in the [acceptance guide](docs/live-acceptance.md).

Deterministic rules work offline. The optional agent is off by default and receives bounded metadata after opt-in. [Privacy](docs/privacy.md) · [Source builds](docs/install.md)

[Usage and development guide](GUIDE.md) · [Report a bug](https://github.com/tcballard/omarchy-sortinghat/issues)

[Licence licensed](LICENSE).

<!-- Preserve links to sections now in the guide. -->
<a id="license"></a>
<a id="remove"></a>
<a id="safety-and-privacy"></a>
<a id="status"></a>
<a id="verification"></a>

[Looking for the previous detailed sections? Open the full guide →](GUIDE.md)
