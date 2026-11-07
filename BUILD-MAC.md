# Experimental Mac source build

This source snapshot derives from **NVForever by LootJunkieAzza and contributors**, originally recorded at `b310430b82216258b85402bd0aeac48677e33bc2`, with identified local Mac/integration changes. It is not an official upstream Mac release.

## Requirements

- Apple Silicon Mac and Xcode command-line tools/Apple SDK.
- Rust toolchain; local development used Rust 1.99.1.
- Network access for the locked Cargo dependencies.
- Your own Fallout: New Vegas and DLC game data, outside this repository.

## Build

From the repository root:

```sh
MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0   cargo +1.99.1 build --release --locked   --manifest-path engine/Cargo.toml -p nvforever --bin nvforever
```

Expected output: `engine/target/release/nvforever`.

Start from the repository root, keeping the supplied schemas in place:

```sh
engine/target/release/nvforever --data "/path/to/your/owned/Fallout New Vegas/Data"
```

Run `--help` for supported options. Keep your existing saves/configuration backed up. Do not point test outputs at original game data. Begin with the base game/DLC; full TTW/Nuclear Sunset integration remains incomplete.

## Evidence boundary

The integrated private development source built locally. This new source-only export excludes captures, private runtime corpora, uncertain binary test vectors and parts of upstream QA tooling. A fresh build from this export has not yet been demonstrated, and bit-identical correspondence to the latest local binary is not asserted. No game binary is distributed here. See `MOJAVE-NATIVE-SOURCE-NOTICE.md`.

Use `UPSTREAM-README.md`, `UPSTREAM-CONTRIBUTING.md`, `LEGAL.md` and upstream sources for original project context. Some upstream documentation describes tooling that this limited export does not contain. Report an exact missing build input rather than copying private data into the source tree.
