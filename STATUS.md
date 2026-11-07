# Development status — 7 November 2026

No public playable release yet.

| Area | Evidence / state |
| --- | --- |
| Architecture | Local release build identifies as Mach-O 64-bit ARM64 |
| Latest local candidate | SHA256 `9083779db3c649038ca187bcb17239e3ef514f27f8f4209d9375303dbb31fac6` |
| Latest candidate gameplay | Focused player acceptance incomplete for this exact build |
| Mouse and sound updates | Adapted merged maintainer PRs [1699](https://github.com/LootJunkieAzza/NVForever/pull/1699) and [1724](https://github.com/LootJunkieAzza/NVForever/pull/1724): unused sound data can be released; finer sensitivity down to 5% |
| Focused validation | 36 focused checks and engine all-target release Clippy passed; one ARM64 release client build completed in 4m22s |
| Bounded runtime | Owned base game + nine add-ons ran on M4 Pro/Metal; normal exit, zero GPU validation errors, no window activation |
| Sound evidence | 53.075s private silent mixer output; radio/effects present, finite samples, peak 0.755001, no clipping values; physical output/latency not checked |
| Save/load evidence | Manual save restored in a fresh process and saved again; checked player fields and position roundtripped; nonblank 512x288 thumbnails |
| Earlier world work | Some rendering/movement exercised; not a complete playthrough |
| Shared compiler changes | Fixed 337-input corpus: 321 pass, 16 errors, no new failures; grouped foreach headers now compile and execute in focused VM checks |
| Full mod experience | TTW/Nuclear Sunset dependency warnings and missing native extension behavior remain |
| Long 15 | Planned; no accepted native gameplay claim |
| New Vegas Drivables | Future vehicles/NPC-traffic integration after the author's release; no native implementation accepted |
| Community source | Source-only export/reproduction work in progress; old archives cannot stand in for current matching source |
| Multiplayer | Future goal; no completed shared-session test |

Next focused acceptance: setup → menu → movement → interaction/combat → sound → save → normal exit → reopen/load, using an owned base-game/DLC profile. Publish hardware, settings, revision and remaining faults with that result.

The new bounded runs establish decoding/mixing and checked save fields, **not** a
human-controlled playthrough, presented FPS or long-session RAM improvement.
The silent capture skipped 257819 frames during startup stalls; it is not evidence
of uninterrupted hardware audio. See the [integration record](docs/handoff/mojave-mac-input-audio-2026-11-07.md).

The latest [grouped foreach change](docs/handoff/mojave-foreach-grouping-2026-11-07.md)
removes compile errors in two MIC item/recipe scripts without rewriting the mods.
The corpus comparison does not execute those real mod bodies. This source change
has not yet replaced the local candidate application listed above.
