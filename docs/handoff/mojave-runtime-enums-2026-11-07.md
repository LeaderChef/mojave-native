# Runtime enumeration arguments — 7 November 2026

## Behaviour repaired

[documented] GECK's [Script Compiler Override](https://geckwiki.com/index.php?title=Script_Compiler_Override)
permits an actor-value code stored in an integer variable inside underscore-prefixed
blocks. The native compiler previously required a literal enum name in that position.
It therefore rejected correct mod helpers before their code could execute.

[observed] The compiler now retains named constants and resolves runtime words through
its existing expression path inside override blocks. Unknown words are not converted
into arbitrary strings. Ordinary-block enum checking, bytecode and runtime budgets
are unchanged. See the [specification](../spec-notes/runtime-enum-arguments.md).

## Evidence

[observed] Before the fix, one authored regression failed and its two controls passed.
All three now pass through the actual compiler, decoder and VM. A changing variable
delivers three distinct enum codes to the host; named constants still work, while
undefined names and ordinary-block runtime enum words remain rejected.

[observed] The same 253-plugin private selection, using winning records as GameData
does, contains 200463 script units and selects 3603
for source compilation. Accepted source units increase from 3456
to 3459; failures decrease from 147 to
144, with no new failing unit. All three affected original
source units compile. The unchanged Vanilla UI Plus and Nuclear Sunset helper bodies
execute six calls in a disposable native Game state; health, action points and a skill
match the direct getter results, without script errors or mod-file edits.

[observed] Release workspace tests: 2292 passed, 8 unchanged ignored,
zero failed. Root and engine all-target release Clippy pass. The smoke-tool Python
suite runs 74 tests with four unchanged skips. The successful 23.46-second silent,
windowless M4 Pro/Metal launch executes an authored file through CompileScript/Call,
verifies two distinct actor values and exits normally without GPU validation errors
or foreground activation. No screenshot is requested. Its function catalogue comes
from the actual app bundle.

[observed] Initial attempts are preserved privately: restricted execution could not
acquire a GPU; the first permissioned run started Metal but the UDF was absent from
the Data VFS; a Data-view setup then refused an overlapping profile path. The final
run uses the same binary and unchanged success markers, with the authored UDF in a
disposable owned-Data view and a separate profile. No original game folder is changed.

The [machine-readable receipt](mojave-runtime-enums-2026-11-07.json) separates these
results from incomplete acceptance work. The [authored UDF fixture](../../tools/smoke/fixtures/runtime-enum-home/)
contains no game data. Run its focused crate regression with
`cargo test --release -p fnv-script --test dynamic_enum`; the full upstream QA export
is still separate from this repository's limited source snapshot.

## Limits

The large UI controller is compile-checked, not visually accepted. Complete Nuclear
Sunset/TTW, current-binary save/reload, hardware audio, presented FPS, long playthroughs,
independent second-Mac build and multiplayer remain open. There is no public playable
binary release. Previous applications, profiles and saves are preserved.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
