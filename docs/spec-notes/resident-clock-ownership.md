# Resident initialization and the playable clock

[observed] Candidate 12's native save contains GameDaysPassed 1000.0002 and FineClock carry 1000.0002024653326. The fresh loaded process later reports 5.00005770 days, despite retaining its hour, dehydration, inventory and checked reference state. The failed measurement is retained in `docs/handoff/mojave-native-clock-2026-11-07.md`.

[observed, engine source] `Residents::Core::think` prepares its game on the first NPC update. It unconditionally seeds GameHour and GameDaysPassed from its startup sky Clock. In playable mode this is the same shared simulation that was already restored by `Runtime::restore_save`. Standalone NPC viewers instead own a separate Game and need their requested initial Clock.

[design] Only the standalone NPC simulation may seed its own calendar. Shared playable NPC preparation sets its package interval and consumes the existing simulation clock, preserving changed globals and the high-precision carry already saved by World. A subsequent NPC update never reinitializes either mode. No save format, original mod, time scale, startup quest or dependency gate changes.

[design] Verify with an authored restored-world regression through the real Core::think entry point, a standalone initialization control, and the unchanged two-process native clock/save probe. This is a correction of clock ownership, not a claim of full TTW compatibility. NVForever by LootJunkieAzza and contributors remains the engine foundation.
