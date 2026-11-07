# Development status — 7 November 2026

No public playable release yet.

| Area | Evidence / state |
| --- | --- |
| Architecture | Local release build identifies as Mach-O 64-bit ARM64 |
| Latest local candidate | SHA256 `3a77f6b1bb0be732f2ae80bf3a2b4876a82ca3c60fef6e9a733eb757ada53bb9` |
| Latest candidate gameplay | Focused player acceptance incomplete for this exact build |
| Mouse and sound updates | Adapted merged maintainer PRs [1699](https://github.com/LootJunkieAzza/NVForever/pull/1699) and [1724](https://github.com/LootJunkieAzza/NVForever/pull/1724): unused sound data can be released; finer sensitivity down to 5% |
| Integrated-source checks | Four inline argument regressions (three red before) now pass; 2292 release workspace tests pass, 8 unchanged ignored; root and engine all-target release Clippy pass |
| Bounded runtime | Owned base game + nine add-ons ran on M4 Pro/Metal; normal exit, zero GPU validation errors, no window activation |
| Session recipe views | Actual MIC Drinks menu plus authored runtime labels/filter checked in a windowless Metal run; no mod bodies rewritten |
| Latest sound evidence | 28.076s private silent mixer output; finite samples, peak 0.721925, no values over full scale; physical output/latency not checked |
| Save/load evidence | Earlier executable `eacf11bd` only: manual save restored in a fresh process and saved again; checked player fields and position roundtripped; nonblank 512x288 thumbnails |
| Earlier world work | Some rendering/movement exercised; not a complete playthrough |
| Shared compiler changes | Fixed 337-input corpus: 321 pass, 16 errors, no new failures; grouped foreach headers now compile and execute in focused VM checks |
| Loaded recipe records | All 291 owned base/DLC recipes, 11 categories, 640 quests and 226 ingestibles enumerated in the native client and after fresh reload; two exact original MIC scripts execute; the actual Drinks menu and a controlled runtime-name/filter probe passed in the recorded earlier candidate |
| Full mod experience | TTW/Nuclear Sunset dependency warnings and missing native extension behavior remain |
| Long 15 | Planned; no accepted native gameplay claim |
| New Vegas Drivables | Future vehicles/NPC-traffic integration after the author's release; no native implementation accepted |
| Community source | Recorded public export built from an empty target on the maintainer's Mac; second-Mac reproduction remains open |
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

The [fresh-source build and launcher relocation](docs/handoff/mojave-source-build-2026-11-07.md) passed on the same Mac.
The new engine runs scripts with the catalogue in the relocated application,
without using the earlier private development schema paths. All 38 launcher-core
checks pass. This does not extend earlier mod/save/audio results to this exact
new executable, and is not a public playable release or second-machine proof.

The [runtime-enum argument repair](docs/handoff/mojave-runtime-enums-2026-11-07.md) reduces failures in the unchanged
253-plugin source selection from 147 to 144, with no new failing unit. Original
UI Plus and Nuclear Sunset helpers return/store the expected actor values in six
private native calls. The new ARM64 build passes its silent windowless UDF fixture;
this is not complete mod/UI, human gameplay or current-binary save/reload acceptance.
