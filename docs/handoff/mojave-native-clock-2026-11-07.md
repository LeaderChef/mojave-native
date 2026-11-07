# Native clock compatibility — candidate 12 not accepted

**Observed, 7 November 2026:** release compilation and source checks pass, but the combined native clock/save probe fails its day-persistence contract. This change stays in draft; candidate 12 was not packaged or selected for the user.

The real native ARM64/Metal client runs the authored startup fixture against owned base-game data plus nine official add-ons. `GetOptionalPatch "bGameDaysPassedFix"` reports the existing NVForever FineClock invariant. Days, hours and Hardcore dehydration advance before and after an explicitly diagnosed disable attempt. Unknown patch names are still false. This is not a JIP version emulation or a newly implemented clock.

**Failed observation:** after setting GameDaysPassed to 1000, the last pre-save sample reports 1000.00018311 days; a fresh process reports 5.00005770 days. Hours and dehydration continue, and inventory, Luck and disabled-reference fields persist, but the day value does not. The original failed receipt is retained; no check, tolerance or original game/mod data was changed to obtain a pass. Cause is not established. Next work must examine the game-global, calendar and engine-save restore boundary rather than changing the expectation.

Both processes exit normally without windows, foreground activation, audio or GPU validation errors. See adjacent JSON for exact samples/checks. Binary SHA256: `a5f7ffc5e4fb6eb472fb459aa5732353c631ed2afdb7c953ecc808b377a8f571`.

Source validation: 2300 workspace tests pass, 8 unchanged ignored; root/engine all-target release Clippy and native release client build pass; smoke Python suite runs 74 tests with 4 unchanged skips; launcher passes 38 checks. The synthetic save roundtrip therefore does not establish the integrated engine/global restore result.

Candidate 11 remains the user's Dock preview. Its [prior inventory/actor/reference save test](mojave-mac-save-roundtrip-11-2026-11-07.md) did not test this high-day clock case; do not imply it excludes the same pre-existing fault. No complete playthrough, TTW/Nuclear Sunset, long-session, physical audio, presented FPS, Long 15 or multiplayer acceptance is claimed.

NVForever by LootJunkieAzza and contributors is the engine foundation. See [clock contract](../spec-notes/native-clock-compat.md). This record includes no original game or mod assets.
