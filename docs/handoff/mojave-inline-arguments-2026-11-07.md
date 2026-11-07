# Inline argument expressions as the only extender syntax — 7 November 2026

An ordinary function call may contain a grouped expression as an argument, with
no separate Let or array declaration in its unit. The native compiler now
recognizes that grammar by the nested call or expression operation itself.
Previously it discarded the parentheses and treated nested getters as unresolved
remote variables, or silently dropped arithmetic operators.

Plain grouped values and signed literals keep the existing classic path. The
existing expression parser and VM still handle evaluation, receiver identity,
enum arguments, argument order, state changes and errors. No original mod source
or bytes are rewritten. The behavior sources and design boundary are in
[the clean-room note](../spec-notes/inline-only-call-arguments.md).

## Evidence

- Four authored compiler/bytecode/IR/VM regressions: one passed and three failed
  before the repair; all four pass afterwards. Nested getters and arithmetic
  need no unrelated Let. Plain grouping stays classic; invalid syntax and unknown
  receivers are rejected rather than hidden.
- 2289 release workspace tests pass, with 8 unchanged ignored;
  root and engine all-target release Clippy pass. Existing goldens, assertions,
  tolerances and script execution budgets were not loosened.
- With the same selected 253 plugins, source fallback compiles 3456 units instead
  of 3429; failures decrease from 174 to 147. These are script-unit counts, not
  working-mod counts or a compatibility percentage.
- Three unchanged original mod units now compile. Direct invocation of the real
  EDGE dialogue result in a disposable native Game state gives the player seven
  items for seeded Luck 7 and gives Sunny one item. Both original perk entries
  restore 41 Health for observed Survival 41, without script errors or skipped
  calls. This tests those result bodies, not dialogue availability, perk triggers,
  animations or a complete playthrough.
- A silent windowless ARM64/Metal client run executes an authored inline-only
  ScriptRunner unit. A separate verifier observes the expected 13 caps from the
  nested getter and arithmetic. Keeping the verifier in a different unit avoids
  accidentally masking the original grammar-selection bug with its Print call.
  Normal exit, zero GPU validation errors and no foreground activation.
- Exact client SHA256: `058b26813021b00fc3ff358da9460e07ac31ea876ebfa75f746cba837038ee75`.

## Reproduce without game data

```sh
cargo test --release -p fnv-script --test inline_only
```

The public fixtures are authored; the original mod text/bytes, owned masters and
private Game probe stay private. The separate local smoke scenario is registered
as `inline-only-arguments`; its complete private catalogue is not this export.

## Remaining work

Other source failures, a malformed classic quest condition, a SWEEP execution
budget failure, dependency gaps and loose-asset assembly remain. This build does
not establish full Nuclear Sunset/TTW, Long 15, human input/audio, presented FPS,
long sessions, current-binary save/reload or multiplayer. Fresh independent
source-export build reproduction also remains pending. Previous applications,
profiles and saves are preserved.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
