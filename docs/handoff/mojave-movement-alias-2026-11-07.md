# Movement source alias — 7 November 2026

[documented] GECK's function-alias category lists `MoveToMarker`. The existing
`MoveTo` catalogue entry describes the marker reference and optional X/Y/Z offsets.
See the [format note](../spec-notes/moveto-marker-alias.md). The repair adds the
alias to opcode `0x109E`; it adds no handler or movement rule.

[observed] The owned selected TTW quest contains 23 stored movement calls. Five
source statements use the alias; resolving those five subjects and their marker
against that plugin's visible records matches five stored `MoveTo` statements.
The complete unchanged quest source now compiles. No source text or compiled
game-derived dump is distributed here.

[observed] The same 253-plugin census selects 3,603 source-fallback units. Successful
compilations increase from 3,465 to 3,466; failed units fall from 138 to 137. Every
remaining failed unit retains exactly the same diagnostics. Ten native Game/World
checks using those actual five references pass: destination cell, coordinates,
rotation, optional offsets and movement/teleport notifications. They are simulation
checks, not a completed TTW quest or a rendered gameplay demonstration.

[observed] The focused compiler tests compare both spellings' bytes/reference
lists for player/local-reference callers, ordinary/override grammar and optional
offsets. Release workspace tests pass: 2,297 passed, eight unchanged ignored, zero
failed. Root and engine all-target release Clippy pass.

[observed] The authored `movement-alias-source` scenario runs in a separate silent,
windowless M4 Pro/Metal process with the new bundled catalogue. Its actor-distance
check prints the success marker and the process exits normally after 32.22 seconds,
without foreground activation or GPU validation errors. At exit, the broader base
game reports one unhandled call; this check does not imply a fully supported script
corpus. No screenshots or original game-folder writes are needed.

[observed] The executable is unchanged from candidate 09 (SHA-256
`a88aead795d1d4a3dc8a39028a8ac79ee4b758caea4bbe20b3760c6b40bff085`).
This is a runtime-loaded catalogue repair, so a different executable is unnecessary.
The new catalogue SHA-256 is
`0f3dacf8e960c6f5cd068e11a382b52be28e5685eb20a2cb7c5083cf30937f2a`.
The [machine-readable record](mojave-movement-alias-2026-11-07.json) separates these
identities and validation scopes. Previous applications, profiles and saves remain.

## Remaining acceptance

[observed] This change preserves the existing player movement implementation; it
does not repair or prove original-game queued-player scheduling parity. Native
input/audio, presented FPS, current-binary save/reload, long playthroughs, full
TTW/Nuclear Sunset and multiplayer remain separate acceptance work.

Reproduce the compiler controls with
`cargo test --release -p fnv-script --test movement_aliases`.
The own smoke fixture is in `tools/smoke/fixtures/movement-alias-home`; the full
upstream smoke runner is not included in this source snapshot.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
