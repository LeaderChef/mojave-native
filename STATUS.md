# Development status — 7 November 2026

**Native Mac preview; full TTW/Nuclear Sunset is not ready. No public playable release.**

| Area | Evidence / state |
| --- | --- |
| Foundation | NVForever Rust engine by LootJunkieAzza and contributors; native 64-bit ARM64/Metal adaptation |
| Latest validated binary | Candidate 12 rebuilt after the resident clock fix; owned base-game data and nine official add-ons |
| Clock/save repair | Shared resident NPC initialization preserves the saved clock; unchanged two-process native probe now passes |
| Current-binary save scope | Days, hours, Hardcore dehydration, inventory delta, Luck and a disabled reference persist; normal Metal exit with no app activation |
| Checks | Real resident-entry regression red before/green after; standalone clock control passes; engine release Clippy/build pass. Unchanged root source retains its prior 2300-test/8-ignored result |
| Earlier candidate 11 | Application and isolated profile preserved; earlier bounded inventory/actor/reference save check belongs to its recorded binary |
| Mod acquisition | Selected Nuclear Sunset downloads/staging complete; acquisition is not native execution |
| Source census | 3466 of 3603 selected units compile; 137 errors remain. This is not a gameplay-completion percentage |
| TTW/Nuclear Sunset | Missing native extension behavior and dependency warnings remain; no gates forged or bypassed |
| Long 15 / vehicles / NPC traffic | Roadmap; no accepted native implementation |
| Multiplayer / shared living world | Roadmap; no completed shared-session test |

[Observed] The [clock/save repair record](docs/handoff/mojave-resident-clock-restore-2026-11-07.md) and [measurements](docs/handoff/mojave-resident-clock-restore-2026-11-07.json) define the latest native evidence. The original failed probe is retained separately. Root and launcher source checks were not repeated for unchanged source.

[Observed] Human-controlled play, visible UI/input, physical audio, presented FPS, long sessions and a full playthrough still need acceptance. The local preview is not the requested completed modded game. Original game/user save paths are untouched.
