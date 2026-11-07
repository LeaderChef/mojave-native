# Native clock compatibility for TTW

- [documented] JIP LN's author describes `bGameDaysPassedFix` as preventing
  GameHour and GameDaysPassed from freezing during real-time play, which also
  stalled Hardcore needs. JIP LN changelog 51.60:
  <https://www.nexusmods.com/newvegas/mods/58277>.
- [documented] GetOptionalPatch reports whether a named patch is enabled:
  <https://geckwiki.com/index.php?title=GetOptionalPatch>.
- [observed] The selected, privately owned TTW startup script requests this patch.
  No original mod text or game-derived bytecode is included in this repository.
- [observed] `fnv-sim::clock::FineClock` already accumulates the low digits in f64
  and publishes the f32 globals from them; it saves those digits and respects
  explicit writes to either global. The native play schedule advances this
  clock, and Game's normal ticks advance Hardcore needs. This work does not
  replace that clock or claim to have created the upstream fix.
- [design] `GetOptionalPatch "bGameDaysPassedFix"` now returns 1 for this existing
  native invariant. `SetOptionalPatch ... 1` requires no state change. This one
  invariant is always on: a request for 0 logs an explicit warning and leaves
  it active. INI requests cannot restore the floating-point freeze. This is a
  deliberate native API difference from JIP's optional toggle, not full JIP
  emulation. The existing creature-spread option remains independently mutable;
  unknown names still return 0. No plugin version is advertised or forged.
- [observed] `tests/game_clock.rs` combines real source dispatch with 60-Hz
  World/Game frames at day 1000 and TimeScale 1, script-visible globals,
  Hardcore dehydration, JSON save/restoration and explicit script hour writes.
  That frame increment is too small to move an f32 at day 1000 on its own.

Full TTW, Stewie's Tweaks and Nuclear Sunset compatibility remain unverified.
The authored test is not a visible gameplay or physical audio/FPS result.
