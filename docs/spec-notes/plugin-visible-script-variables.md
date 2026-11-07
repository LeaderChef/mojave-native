# Source fallback uses plugin-visible script variables

- [observed] GameData keeps only winning executable units. Its source fallback
  currently also constructs the name-resolution index from those winners.
  A later SCPT override can rename a variable at the same authored index.
  Earlier helper source then incorrectly sees the later-only name.
- [documented] `unit.rs` reads SCPT variable names and indices from the Layer 1
  schema's SLSD/SCVR definitions. `compile/env.rs` already defines visibility as
  the compiling plugin and its declared masters; this change applies that model
  to complete historical SCPT metadata. No new original-engine layout is inferred.
- [design] Read historical SCPT metadata lazily only when source fallback is
  needed. Keep the runtime winner list unchanged. Preserve original numeric
  variable indices; never rename a mod variable or invent an alias.
- [design] Missing visible metadata must fail lookup, rather than substituting
  a future plugin's definition. Sort script definitions explicitly by plugin.
- [hypothesis] The GECK authoring context is the active plugin and its masters,
  as recorded by the existing compiler environment. Authored fixtures and the
  private TTW source/compiled-variable records check consistency, not full GECK
  parity or complete companion gameplay. An actual mod that changes slot meaning
  can still be incompatible and must not be declared repaired by compilation.

Acceptance: a multi-plugin fixture reproduces the refusal before the fix; the
unchanged fixture then compiles the earlier helper, executes its original slot
against the winning runtime script, and rejects later-only names. The selected
profile source census is compared using the same winning-unit selection.
No game payload is committed. Full TTW and other gameplay remain separate work.

## Evidence and compatibility boundary

- [observed, authored fixture] Before the repair, the earlier helper fails with
  `unresolved_remote_variable`; an invalid helper using the future name compiles
  instead. After the repair, the earlier helper writes its original slot through
  native `Game`, the later runtime definition remains the winner, and the
  future-only helper is refused. The other slot stays unchanged.
- [observed, private selected-profile probe] The unchanged 253-plugin profile
  selects 3,603 fallback units. Successful compilations increase from 3,466 to
  3,467; failures fall from 137 to 136, with no new failed unit and no changed
  diagnostics on the remaining failed units. Production `GameData` reports the
  same counts. Original plugin files are unchanged.
- [observed, private owned compiled bytes and record metadata] The recovered TTW
  helper's remote write keeps its original slot 7. In the selected runtime
  override, that slot is named `rWeapon`, whereas the earlier declaration calls
  it `IsFollowingShort`. This is a separate record-level compatibility conflict:
  source compilation matching the original bytes does not make those two
  meanings interchangeable. No automatic name alias or slot remapping is made.
- [required follow-up] Resolve that selected-profile conflict using the mods'
  documented intended behavior and a separate compatibility patch, then exercise
  follower dismissal and persistence. Compilation, numeric-slot fidelity and the
  authored native runtime regression do not establish TTW follower gameplay or
  complete Nuclear Sunset compatibility.
