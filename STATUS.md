# Development status — 7 November 2026

**Native Mac preview; full TTW/Nuclear Sunset is not ready. No public playable release.**

| Area | Evidence / state |
| --- | --- |
| Foundation | NVForever Rust engine by LootJunkieAzza and contributors; local 64-bit ARM64/Metal adaptation |
| User preview | Candidate 11, launcher 0.2.13; added to the user's Dock; base-game data and nine official add-ons found |
| Candidate 11 save/load | Two native processes retain inventory delta, Luck and a disabled reference. This did not test high-day clock persistence |
| New candidate 12 | Release build passes; kept in draft after an integrated clock/save test failed |
| Failure | GameDaysPassed is 1000.00018311 before save and 5.00005770 after fresh load; hours/needs and other checked state persist |
| Source checks | 2300 workspace tests pass, 8 unchanged ignored; root/engine Clippy and native build pass |
| Mod acquisition | Selected Nuclear Sunset downloads/staging complete; this is not native gameplay acceptance |
| Source census | 3466 of 3603 selected source units compile; 137 errors remain; compilation is not working gameplay |
| TTW/Nuclear Sunset | Missing native extension behavior and dependency warnings remain; no gates forged or bypassed |
| Long 15 / vehicles / NPC traffic | Roadmap; no accepted implementation |
| Multiplayer / living world | Roadmap; no completed shared-session test |

The [failed candidate 12 observation](docs/handoff/mojave-native-clock-2026-11-07.md) and [measurements](docs/handoff/mojave-native-clock-2026-11-07.json) are retained. The limited [candidate 11 save/reload record](docs/handoff/mojave-mac-save-roundtrip-11-2026-11-07.md) belongs to its exact binary.

Human-controlled play, visible UI/input, physical audio, presented FPS, long sessions and a full playthrough still need acceptance. The local Dock entry is a preview, not the requested completed modded game. Original user saves and prior applications are preserved.
