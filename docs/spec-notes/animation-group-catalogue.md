# Documented animation group names in script bytecode

[Documented] The GECK [AnimGroups](https://geckwiki.com/index.php?title=AnimGroups)
reference gives the animation group code/name pairs from 0 (Idle) through 244
(JumpLandRight). These are animation argument values, not function opcodes.
Some combat animations need an alerted actor. A recognized argument does not
guarantee that the particular actor has the requested animation clip.

[Observed] Before this repair, `crates/fnv-script/data/encodings.json` included
only 14 names observed in the shipped masters. The native source compiler rejected
documented names outside that subset, including TurnRight, DynamicIdle, JumpStart,
JumpLoop, AimIS, AttackLeft and BlockHit. This prevented compilation of otherwise
valid movement, aiming and melee scripts in the selected mod profile.

[Design] Complete the shared `anim_group` catalogue with the documented pairs.
Preserve all previously observed pairs. Both compiler forms and the bytecode
decoder use the same catalogue, so the existing game's animation request carries
the documented code and a name that the renderer can resolve. Unknown names still
fail compilation; no new function opcode, silent fallback or per-mod rule is added.

[Validation scope] Compile/decode/VM checks cover named arguments and all start
flags; actual game-path probes check the queued request's reference, group code,
name and start mode. Source-corpus checks retain the original selected plugins and
winning records. These checks do not prove that every animation is present,
renders correctly, blends correctly, or that Enhanced Movement is fully playable.
Those require the later visible first/third-person test.

[Provenance] Only the public GECK reference and the owned original mod data were
used to establish this change. No Bethesda/Gamebryo implementation, extender source,
DLL implementation or game-derived payload is included in this note.
