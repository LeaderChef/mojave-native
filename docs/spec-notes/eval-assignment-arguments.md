# Assignments passed to Eval and TestExpr

- [documented] `Eval` reduces an NVSE expression to a boolean. `TestExpr`
  evaluates an expression and returns whether evaluation succeeded; assignments
  are permitted in NVSE-aware expressions, including these functions.
  Sources: https://geckwiki.com/index.php/Eval,
  https://geckwiki.com/index.php/TestExpr,
  https://geckwiki.com/index.php/NVSE_Expressions.
- [observed] In the owner's unchanged mod data, the coordinates callback in
  Just Assorted Mods contains standalone `eval` calls assigning numeric and
  reference locals. The native compiler parses the assignment outside the call
  and refuses the line as `not_a_call`; the source fallback then retains foreign
  function-block bytecode, which the native runtime cannot call. Private
  evidence: `work/native-script-faults-01/jbt-compile.json`. No original mod
  source or bytes are included in this repository.
- [decision] The single expression argument of Eval/TestExpr includes assignment
  precedence. Their existing VM behavior still determines boolean conversion,
  successful zero-valued assignment, side effects and suppressed TestExpr errors.
  Other function argument boundaries are unchanged. Invalid assignment targets
  and missing expressions must still be rejected.
- [observed] A separate runtime error in the selected profile has a classic
  `GetStage` condition with surplus operands in its source and compiled data.
  This repair must not discard those operands or turn an invalid expression into
  success (see `scripts.md` and the existing malformed-expression VM tests).
