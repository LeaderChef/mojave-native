# Native Eval/TestExpr assignment arguments — 7 November 2026

The native compiler now keeps assignment inside the single expression accepted
by Eval and TestExpr. Previously it parsed a statement such as `Eval n = 4`
as an assignment to the return of Eval, rejecting the statement as `not_a_call`.
This could reject an entire source-fallback function and leave a mod callback
with foreign bytecode that the native runtime could not call.

The existing VM still controls boolean conversion, assignment side effects and
TestExpr error handling. Other functions retain their existing argument
boundaries. The change adds no opcode placeholders and rewrites no mod data.
The clean-room behavior sources are in
[the specification](../spec-notes/eval-assignment-arguments.md).

## Evidence

- Two new source/compiler/bytecode/IR/VM regressions failed before the repair and
  pass afterwards. Numeric, chained and reference assignments work; assigning
  zero remains false through Eval and a success through TestExpr. Invalid array
  assignment remains a caught TestExpr error.
- Release workspace: 2285 passed, 8 unchanged ignored.
  Root and engine all-target release Clippy pass. No assertions, goldens or
  tolerances were loosened.
- In the same selected 253-plugin profile, native source fallback compiles
  3429 units, compared with 3424 in the prior recorded executable. Remaining
  source compilation failures decrease from 179 to 174. This is not a count of
  working mods or a compatibility percentage.
- The unchanged Just Assorted Mods coordinates handler has a native Function
  block and no undecoded pieces. Its actual idle and active/no-target branches
  execute in the native Game VM. The latter updates both output coordinates to
  -1 through the existing UI functions, in an explicitly synthetic two-trait UI
  host fixture. This does not prove the complete JAM HUD is installed.
- The Mac ARM64/Metal client also runs the selected profile windowlessly and
  silently; the former per-frame `has no Function block` error is absent and
  the authored Eval assignment ScriptRunner succeeds. The process exits normally
  with zero GPU validation errors and no foreground activation.
- Exact native executable SHA256: `118995d82827ef2af95e0ffedbae44a8dbf1a877ad6190ea9070e7b60f95fa87`.

## Reproduce the public regression

```sh
cargo test --release -p fnv-script --test eval_assignment
```

These tests contain only authored fixtures and need no game data. The private
native run contains owned game/mod data and is not distributed. Its new smoke
fixture is registered locally as `eval-assignment-arguments`; the private smoke
catalogue remains separate from this source export.

## Boundaries

Another selected-profile quest still stops at an invalid classic GetStage
condition with surplus operands. The existing malformed-expression refusal is
preserved; this change does not silently discard its operands. Other source
compilation and dependency failures remain. Full loose-asset assembly,
Nuclear Sunset/TTW gameplay, Long 15, current-binary fresh save/reload, human
input/audio, presented FPS, long sessions and multiplayer remain unaccepted.
Fresh external source-export build reproduction is also pending.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later. Original mod
files remain unchanged; no game/mod payload, private saves or credentials are
included. Existing candidate applications and profiles remain available.
