# Mojave Native — source-only candidate

This source snapshot is derived from NVForever by LootJunkieAzza and its contributors.
Upstream: https://github.com/LootJunkieAzza/NVForever
NVForever is licensed GPL-3.0-or-later; LICENSE, LEGAL.md, CONTRIBUTING.md and original notices are preserved.
Mojave Native is a community macOS port effort, independent of Bethesda, Obsidian and upstream project endorsement.

This candidate contains verbatim selected source/build inputs, schemas and project documentation. It contains no owned game/DLC content, mod archives, captures, runtime corpus, authentication material or compiled game binary. Each user must supply their own lawfully obtained game data separately.

Build entry point (not executed for this export): `cd engine && cargo build --release --locked --bin nvforever`.
The original root/engine manifests and lockfiles are retained. The two required NVForever program icons are upstream project assets, not rendered game captures.

Private/capture/parity/runtime trees, nonessential tooling, binary test vectors of unverified provenance, and privacy-flagged documents are excluded. As a result the full upstream QA workflow/tool collection is not represented here. No source-to-binary identity, complete test-suite pass, or successful fresh build is asserted by this candidate. The upstream documentation describes upstream behavior and does not establish Mojave Native runtime completeness.
