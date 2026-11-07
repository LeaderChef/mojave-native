# Inline argument expressions without another extender statement

- [documented] xNVSE 6.1.0 allows an NVSE expression in parentheses as a
  function argument. The documented example is an ordinary actor-value setter
  receiving a calculation containing an actor-value getter. It does not require
  a separate Let, array declaration or compiler-override block in the unit.
  Sources: https://geckwiki.com/index.php?title=Inline_Expression and
  https://geckwiki.com/index.php/GetActorValue (GetAV alias and calling actor).
- [observed] Three unchanged units in the owner's selected profile use exactly
  this grammar as their only extender syntax: an item-count argument in an
  EDGE dialogue result and actor-value restoration amounts in two perk entries.
  The existing compiler drops parentheses, reads the nested getter as a remote
  variable, and refuses the unit. Private evidence is in
  work/native-expression-calls-01/selected-original-units-before.json; its mod sources
  and compiled bytes are not published.
- [decision] A call line containing a grouped function call or nontrivial
  expression is itself sufficient evidence of extender grammar. The existing
  expression parser, argument encodings and VM evaluate that line; no new
  placeholder commands or mod-specific rewriting is introduced.
- [observed] Plain grouping of an atom, including a signed numeric literal,
  is also accepted by the vanilla compiler, which discards the parentheses
  (scripts.md section 3.4 and tests/compile.rs). Preserve that existing path
  when the group contains no expression operation or nested function.
- [decision] This recognition only selects a grammar. Unknown references,
  missing operands, unmatched groups and other invalid expression syntax still
  fail compilation. Function argument order, receiver identity, enum types,
  state changes and runtime error behavior remain controlled by the existing
  compiler/VM. Do not suppress the separate malformed GetStage condition or
  increase script execution budgets as part of this repair.
