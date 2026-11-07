# Development status — 7 November 2026

No public playable release yet.

| Area | Evidence / state |
| --- | --- |
| Architecture | Local release build identifies as Mach-O 64-bit ARM64 |
| Latest local candidate | SHA256 `058b26813021b00fc3ff358da9460e07ac31ea876ebfa75f746cba837038ee75` |
| Latest candidate gameplay | Focused player acceptance incomplete for this exact build |
| Mouse and sound updates | Adapted merged maintainer PRs [1699](https://github.com/LootJunkieAzza/NVForever/pull/1699) and [1724](https://github.com/LootJunkieAzza/NVForever/pull/1724): unused sound data can be released; finer sensitivity down to 5% |
| Latest validation | Four inline argument regressions (three red before) now pass; 2289 release workspace tests pass, 8 unchanged ignored; root and engine all-target release Clippy pass |
| Bounded runtime | Owned base game + nine add-ons ran on M4 Pro/Metal; normal exit, zero GPU validation errors, no window activation |
| Session recipe views | Actual MIC Drinks menu plus authored runtime labels/filter checked in a windowless Metal run; no mod bodies rewritten |
| Latest sound evidence | 28.076s private silent mixer output; finite samples, peak 0.721925, no values over full scale; physical output/latency not checked |
| Save/load evidence | Earlier executable `eacf11bd` only: manual save restored in a fresh process and saved again; checked player fields and position roundtripped; nonblank 512x288 thumbnails |
| Earlier world work | Some rendering/movement exercised; not a complete playthrough |
| Shared compiler changes | Fixed 337-input corpus: 321 pass, 16 errors, no new failures; grouped foreach headers now compile and execute in focused VM checks |
| Loaded recipe records | All 291 owned base/DLC recipes, 11 categories, 640 quests and 226 ingestibles enumerated in the native client and after fresh reload; two exact original MIC scripts execute; the actual Drinks menu and a controlled runtime-name/filter probe pass in the latest client |
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
The corpus comparison alone does not execute those real mod bodies. A subsequent
bounded native probe now executes both unchanged MIC files and verifies all
59 name changes and 122 category changes. The recorded recipe-menu candidate also consumes them
in its recipe menu; see the [menu handoff](docs/handoff/mojave-recipe-menu-2026-11-07.md).
Old applications and profiles are preserved. Complete MIC/Nuclear Sunset gameplay
and runtime-name persistence remain unaccepted.

The [Eval/TestExpr callback repair](docs/handoff/mojave-eval-assignments-2026-11-07.md) removes five source-compile failures
in the selected 253-plugin profile (179 → 174). The actual JAM callback now
executes; unrelated quest/dependency errors remain. The latest client has not
yet repeated earlier save/reload or menu presentation checks.

The [inline-only argument repair](docs/handoff/mojave-inline-arguments-2026-11-07.md) restores 27 additional source units
(174 → 147 failures). Actual dialogue-result inventory amounts and two perk
healing results match their getters in a disposable native Game state. A silent
Metal run separately verifies the authored getter/arithmetic inventory fixture.
Current-binary save/reload and human gameplay remain unaccepted.
