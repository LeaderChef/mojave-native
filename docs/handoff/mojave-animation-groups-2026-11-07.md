# Animation-group catalogue — 7 November 2026

[documented] The [GECK AnimGroups table](https://geckwiki.com/index.php?title=AnimGroups)
assigns names to 245 group codes, including AimIS, Attack3, JumpStart and JumpLoop.
The native script catalogue held only fourteen observed entries. This change adds
the documented pairs while preserving all fourteen original codes. It adds no
function opcode, renderer behaviour or game/mod data. See the
[format note](../spec-notes/animation-group-catalogue.md).

[observed] Authored regression tests first failed on the missing names. They now
exercise compilation, decoding and the VM for nine representative names and all
three start flags. A native Game test checks the twenty-seven requests' reference,
code, name and start mode. IsAnimPlaying remains false until an explicit playback
report arrives; the test does not pretend queue submission is playback.

[observed] In the unchanged 253-plugin private selection, 3,603 source units are
selected from winning records. Compiled units increase from 3,459 to 3,465; failures
decrease from 144 to 138, with no new failing unit. The six repaired source units
belong to Enhanced Movement (two), Into The Deep, Just Assorted Mods,
Point That Somewhere Else and Simple Melee Attack Cancel and Parry. Original mod
files remain untouched. Their source text and private game-derived corpus are not
published.

[observed] Release workspace tests pass: 2,295 passed, eight unchanged ignored,
zero failed. Root and engine all-target release Clippy pass and the ARM64 client
builds. The smoke-tool suite passes 74 tests with four unchanged skips. The
[authored smoke scenario](../../tools/smoke/scenarios.toml) executes in a silent,
windowless 32.71-second M4 Pro/Metal run using the actual packaged binary and its
bundled catalogue. It exits normally without foreground activation or GPU errors.
The full upstream smoke runner is not included in this limited source snapshot.
An initial Python 3.9 invocation could not import tomllib; the suite was run using
the existing Python 3.14 installation, without changing its expectations.

The [machine-readable evidence](mojave-animation-groups-2026-11-07.json) records the
scope and limits. Reproduce the focused compiler test with
`cargo test --release -p fnv-script --test animation_groups` and the Game test with
`cargo test --release -p fnv-game --test synthetic documented_animation_groups_reach_the_existing_playback_interface`.

## Still open

Rendered animation, blending, full Enhanced Movement/Nuclear Sunset gameplay,
visible input/FPS, hardware audio, current-binary save/reload, long playthroughs,
independent second-Mac reproduction and multiplayer remain unaccepted. These tests
prove name resolution and dispatch through the existing interface, not complete
mod parity. Previous applications, profiles and saves are preserved.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
