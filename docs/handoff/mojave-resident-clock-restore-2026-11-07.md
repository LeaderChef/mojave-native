# Preserve the restored clock during resident initialization

[observed] The first resident NPC update overwrote the shared playable clock with the authored startup sky clock after load. The actual save retained the correct global and FineClock carry. The failing native measurement and its original binary remain recorded in [candidate 12's earlier failed probe](mojave-native-clock-2026-11-07.md); that failure was not relabelled a pass.

[design] Only the standalone NPC viewer seeds its own private world. Shared NPC preparation now consumes the simulation clock already established by new-game/load/script behavior. The existing FineClock, save format, time scale, mod content and dependencies are unchanged.

[observed] A regression through the actual `Core::think` entry point fails before the fix (shared clock becomes hour 6/day 5 instead of the restored hour 12/day 1000) while its standalone control passes. Both pass after the fix, including unchanged high-precision frame carry and one-time standalone initialization. Engine all-target release Clippy and the ARM64 release build pass. The root code is unchanged from its preceding 2300-test/8-ignored passing run; it was not rerun for this engine-only repair.

[observed] The exact same two-process windowless Metal probe was repeated with the rebuilt binary and a separate home. No expectations, comparison tolerances or original files were changed. Measured clock/Hardcore values:

| Sample | Days passed | Hour | Dehydration |
| --- | --- | --- | --- |
| A | 1000.0 | 12.00041771 | 10.15987011 |
| B | 1000.00012207 | 12.00233555 | 10.85710718 |
| C | 1000.00018311 | 12.00475216 | 11.75743901 |
| L | 1000.00024414 | 12.00625229 | 12.46692609 |

[observed] All probe checks pass: days/hours/needs advance, an attempted disable is explicitly diagnosed, the real clock getter remains enabled, and a fresh process restores days/needs, the +3 Stimpak delta, Luck 9 and a disabled reference. Both processes exit normally on M4 Pro/Metal without a native window, foreground activation or GPU validation error. The saved file is unchanged by loading. The bundled launcher source is unchanged and retains its preceding 38 passing checks; binary hash and signature are checked after replacement.

Binary SHA256: `31a2da9f76fc4e8c8d6cc79079e92cbc9fddddcdd1f1747784c8294c3b92c861`.

[observed] This is bounded native base-game + nine official add-on evidence. It does not prove visible controls/UI, hardware audio, presented FPS, long sessions, full TTW/Nuclear Sunset/Long 15 gameplay or multiplayer. No dependency versions were forged. Original user saves and previous applications/profiles remain separate.

[documented] NVForever by LootJunkieAzza and contributors is the engine foundation; [clock ownership note](../spec-notes/resident-clock-ownership.md) describes the native design.
