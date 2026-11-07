# Native script-error recovery

- [documented] JIP LN's author describes `bNoFailedScriptLocks` as preventing a
  failed script from being permanently disabled until the game restarts. Primary
  reference: [JIP LN changelog v52.00](https://www.nexusmods.com/newvegas/mods/58277).
  [SetOptionalPatch](https://geckwiki.com/index.php/SetOptionalPatch) and
  [GetOptionalPatch](https://geckwiki.com/index.php?title=GetOptionalPatch) document
  the optional patch set/query API. No extender implementation is used.
- [observed, native source] `Game::exec` reports a VM error and unwinds the
  nesting depth. It does not add a disabled marker. The normal `Game::tick`
  scheduler continues to select a running quest and its persistent instance on
  subsequent due ticks. This is existing NVForever behavior, not a newly written
  scheduler or blanket claim about original-engine parity.
- [design] Report that always-on native invariant through the named optional
  query. A request to disable it logs a warning and leaves it active; native
  correctness is not an option to reintroduce permanent failure. This deliberately
  differs from the original toggle. Unknown patches remain unsupported, the
  independent creature-spread switch remains mutable, and no extender version is
  advertised. Explicit `StopQuest`, menu/delay rules and `ToggleScripts` still apply.
- [observed, authored fixture] `tests/script_error_recovery.rs` schedules a real
  quest whose conditional infinite loop reaches the VM step budget. The error
  remains visible, previous local writes survive and subsequent statements do
  not execute. Changing the condition permits the next normal tick to complete.
  A second failure followed by real World/Game JSON save/restore also recovers.
  The same fixture exercises normal and strict execution, an explicit quest stop
  and the global script toggle. It does not alter the original mod or suppress an
  error. The capability regression fails before the mapping change while the
  underlying scheduler regression already passes.

This establishes a bounded native capability, not complete TTW, Stewie/ROOGNVSE,
Nuclear Sunset, physical audio, visible controls or performance acceptance. No
game/mod payload is part of the authored fixture.
