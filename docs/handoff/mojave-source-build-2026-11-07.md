# Fresh public-source build and launcher relocation — 7 November 2026

## What passed

[observed] The published source tree at `fe28fc83ad9c5e016bd7f19b570f03bbe948a744` was exported,
unpacked into a new directory and built with an empty Cargo target directory.
No previous compiled engine or target artifacts were copied. All 1546
source-file hashes remained unchanged. The build took 14.89 minutes
and produced a Mach-O 64-bit ARM64 executable, SHA256
`dc037352e65baa74241cd3fe354da503158640b8c75bab657eae1ec870a2ffbe` (266369720 bytes).

[observed] Host: Apple M4 Pro/Metal, macOS 26.3.1 (25D2128),
Rust 1.99.1, Apple macOS SDK 15.5, deployment target 13.0, two build jobs,
incremental compilation disabled. The existing Cargo dependency download cache
was used offline. This is a fresh **same-host** build, not a second contributor's
machine, a cold network dependency installation or a bit-reproducible-build claim.
The [machine-readable receipt](mojave-source-build-2026-11-07.json) pins the inputs.

[observed] The new executable's `--help` exits normally. Its dynamic dependencies
are Apple system libraries/frameworks. A single 27.52-second silent,
windowless Metal run from a relocated app bundle loads the owner's base game and
nine official add-ons, runs game scripts and exits normally, without foreground
activation or GPU validation errors. No game data is supplied by this repository.

## Launcher correction

[observed] The nested application engine and its schemas were in different
bundle directories. The old launch plan set fnv/nif schemas but left the script
function catalogue to the engine's build-time source path. The maintainer's
retained source directory masked that packaging dependency.

[decision] The launch plan now requires `script/functions.json` from its bundled
schemas, clears an inherited catalogue override and explicitly passes the bundled
path. A missing file refuses launch before creating a profile. The plan follows
the bundle after relocation. Its new path assertion failed before the fix;
all 38 launcher-core checks pass after it. The relocated native run uses this
actual launch-plan path and a byte-matching copy of the public catalogue.
The [reusable launcher core](../../launcher/core/README.md) contains authored,
game-data-free checks; the themed application is not a public binary release.

## Limits and next evidence

The code/manifests/schemas used for this build are unchanged by this documentation
and launcher-core publication. Earlier integrated-source tests and mod-result
probes are separately recorded; the export omits some upstream/private QA inputs,
and a full QA suite was not rerun against this export. Existing applications,
profiles and saves are preserved. Human input, hardware audio, presented FPS,
current-binary save/reload, long playthroughs, full TTW/Nuclear Sunset/Long 15 and
multiplayer remain unaccepted. A clean build on another contributor's Mac is
still requested in [issue 1](https://github.com/LeaderChef/mojave-native/issues/1).

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
