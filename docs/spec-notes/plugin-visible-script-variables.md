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
- [required follow-up] A later mod can deliberately change the meaning of an
  original numeric slot. Source fidelity alone cannot repair such a semantic
  conflict. Resolve the documented intent in a separate compatibility patch and
  exercise follower behavior and persistence before claiming gameplay acceptance.
- [scope] Only authored fixtures are included. No extracted game scripts, private
  record contents or game/mod payload are distributed in this change. Native
  compilation does not establish full TTW or Nuclear Sunset compatibility.
