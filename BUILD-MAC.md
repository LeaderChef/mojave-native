# Build Mojave Native on Apple Silicon

Based on **NVForever by LootJunkieAzza and contributors**, GPL-3.0-or-later.
This community adaptation is not an official upstream Mac release.

## Requirements

- Apple Silicon Mac and Xcode command-line tools/Apple SDK.
- Rust 1.99.1 for the recorded build (`rustup toolchain install 1.99.1` if needed).
- Network access on the first Cargo dependency download, or a populated dependency cache.
- Your own New Vegas/DLC data for running the game; game data is not needed to compile it.

## Build from the repository root

```sh
MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo +1.99.1 build --release --locked --manifest-path engine/Cargo.toml -p nvforever --bin nvforever
```

Output: `engine/target/release/nvforever`. If you explicitly set `CARGO_TARGET_DIR`,
the executable is instead under that directory's `release` folder.
Use `--offline` only when the locked dependencies are already cached.

Start from the repository root, with schemas kept in place:

```sh
NVF_PLAY_SCRIPT_FUNCTIONS="$PWD/schemas/script/functions.json" engine/target/release/nvforever --data "/path/to/your/owned/Fallout New Vegas/Data" --esm-schema "$PWD/schemas/fnv" --nif-schema "$PWD/schemas/nif"
```

Run `--help` for supported options. Keep existing saves/configuration backed up;
begin with base game/DLC. Do not put test outputs in original game data.
Full TTW/Nuclear Sunset integration remains incomplete.

## Recorded build and portability

A fresh same-host release build of the recorded public tree passed, followed by
a silent ARM64/Metal startup from a relocated local application. It used an empty
target directory and the cached Cargo dependencies; no compiled game was copied.
See the [pinned build/startup receipt](docs/handoff/mojave-source-build-2026-11-07.md).
A second Mac, cold network setup, human playthrough and bit-identical build are
separate acceptance work. There is no public playable application release yet.

Packagers must supply **all** schemas, including `script/functions.json`, and
bind the script function catalogue to the bundled copy rather than a developer's
source path. The [launcher-core checks](launcher/core/README.md) cover relocation
and missing-catalogue rejection without opening a window.

The export excludes private runtime corpora and parts of upstream QA tooling.
The recorded client build needs no private fixture; the full upstream test suite
and all tools are not represented by this limited export. Report exact missing
inputs rather than copying private/game data. Retain `LICENSE`, `LEGAL.md`,
upstream credit and dependency notices. See `MOJAVE-NATIVE-SOURCE-NOTICE.md`.
