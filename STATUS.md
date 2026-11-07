# Development status — 7 November 2026

No public playable release yet.

| Area | Evidence / state |
| --- | --- |
| Architecture | Local release build identifies as Mach-O 64-bit ARM64 |
| Latest local candidate | SHA256 `eacf11bd5ba34b5d4fc784a6bb8b766854fade60b513433afe8a1c224a3d40b7` |
| Latest candidate gameplay | Focused player acceptance incomplete for this exact build |
| Mouse and sound updates | Adapted merged maintainer PRs [1699](https://github.com/LootJunkieAzza/NVForever/pull/1699) and [1724](https://github.com/LootJunkieAzza/NVForever/pull/1724): unused sound data can be released; finer sensitivity down to 5% |
| Latest validation | Loaded-form parser/VM regression failed before the fix and passed after it; 2283 release workspace tests pass, 8 unchanged ignored; root and engine all-target release Clippy pass |
| Bounded runtime | Owned base game + nine add-ons ran on M4 Pro/Metal; normal exit, zero GPU validation errors, no window activation |
| Latest sound evidence | 28.076s private silent mixer output; finite samples, peak 0.721925, no values over full scale; physical output/latency not checked |
| Save/load evidence | Manual save restored in a fresh process and saved again; checked player fields and position roundtripped; nonblank 512x288 thumbnails |
| Earlier world work | Some rendering/movement exercised; not a complete playthrough |
| Shared compiler changes | Fixed 337-input corpus: 321 pass, 16 errors, no new failures; grouped foreach headers now compile and execute in focused VM checks |
| Loaded recipe records | All 291 owned base/DLC recipes, 11 categories, 640 quests and 226 ingestibles enumerated in the native client and after fresh reload; actual MIC menus still unaccepted |
| Full mod experience | TTW/Nuclear Sunset dependency warnings and missing native extension behavior remain |
| Long 15 | Planned; no accepted native gameplay claim |
| New Vegas Drivables | Future vehicles/NPC-traffic integration after the author's release; no native implementation accepted |
| Community source | Source-only export/reproduction work in progress; old archives cannot stand in for current matching source |
| Multiplayer | Future goal; no completed shared-session test |

Next focused acceptance: setup → menu → movement → interaction/combat → sound → save → normal exit → reopen/load, using an owned base-game/DLC profile. Publish hardware, settings, revision and remaining faults with that result.

The new bounded runs establish decoding/mixing and checked save fields, **not** a
human-controlled playthrough, presented FPS or long-session RAM improvement.
Silent captures do not establish uninterrupted hardware audio. The prior input/audio
[integration record](docs/handoff/mojave-mac-input-audio-2026-11-07.md) and latest
[loaded-record correction](docs/handoff/mojave-loaded-types-2026-11-07.md) describe their separate scopes.

The latest [grouped foreach change](docs/handoff/mojave-foreach-grouping-2026-11-07.md)
removes compile errors in two MIC item/recipe scripts without rewriting the mods.
The corpus comparison does not execute those real mod bodies. The latest local
client includes that compiler correction plus the loaded-record repair; old
applications and profiles are preserved. The complete MIC interface is still not
accepted.
