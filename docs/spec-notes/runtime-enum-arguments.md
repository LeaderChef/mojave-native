# Runtime enumeration arguments in compiler-override blocks

- [documented] GECK's [Script Compiler Override](https://geckwiki.com/index.php?title=Script_Compiler_Override)
  selects expression arguments by prefixing a block name with `_`. Its actor-value
  examples accept a runtime integer variable as the argument to GetAV, including a
  variable updated in a loop. This is public behaviour documentation, not extender code.
- [observed] Before this repair, the native compiler treats every single word in an
  enumeration parameter as a literal name. An authored `_GameMode` loop therefore fails
  with `unknown_enum_name` for its declared integer variable. The regression executes
  compiled bytes through the existing decoder and VM; a host records each evaluated
  argument. The pre-repair run fails while literal and vanilla rejection controls pass.

## Implementation boundary

- [documented] Named enumeration constants and numeric codes remain valid. Within an
  override block, a word which is not such a literal must use the existing expression
  parser so declared runtime values can reach the existing expression evaluator.
- [observed] The compiler already tracks override blocks and argument precedence. No
  new bytecode, invented opcode, native DLL loader or second execution path is required.
- [observed] Unknown names must still fail resolution. Ordinary blocks retain literal
  enumeration checking. No missing value is replaced with zero and no runtime budget
  or malformed-expression tolerance is changed.
- [observed] `crates/fnv-script/tests/dynamic_enum.rs` covers changing runtime arguments,
  literal names and unresolved-name rejection, plus unchanged ordinary-block rejection.

## Acceptance scope

- [observed] Targeted regression, the unchanged selected-plugin compiler census and
  execution of original affected user-owned helper scripts are separate checks. A
  compile result alone does not establish that their gameplay effects work.
- [hypothesis] Some mod scripts may still fail after this shared correction because of
  other unsupported calls or source errors; the complete modpack is not thereby accepted.
