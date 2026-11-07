# Grouped foreach headers — 7 November 2026

## Problem and change

[observed: private, unchanged selected Nuclear Sunset inputs] Two restart-stage
MIC item/recipe scripts put parentheses around the entire foreach header. The
compiler split the iterator and collection at the binding arrow before accounting
for those parentheses. It rejected the opening half as a missing closing bracket.

[hypothesis: native compatibility extension] The compiler now removes only complete,
balanced enclosing pairs before resolving the iterator and collection. Nested
collection expressions and inline iterator declarations keep their existing
behavior. Unmatched grouping, trailing tokens and non-variable iterator targets
still produce errors. The change is generic and does not branch on mod names.

[documented: [ForEach](https://geckwiki.com/index.php?title=Foreach) and
[NVSE Expressions](https://geckwiki.com/index.php/NVSE_Expressions)] The primary
documentation establishes expression grouping and collection iteration. It does
not prove that the original compiler accepts grouping around the whole header;
this report does not claim measured original-engine parity for that extension.

## Evidence

[observed: focused compiler/VM checks] Five foreach tests passed, including three
new regressions. They execute grouped array and string loops, verify break and
continue, check inline declarations, and reject six malformed headers.

[observed: one compile-only corpus comparison] The exact same 337 eligible loose
scripts were compiled in the selected 253-plugin context. The prior result was
319 successes and 18 failures; the new result is **321 successes and 16 failures**.
Only the two expected MIC scripts changed from failure to success. No previously
successful input regressed. Original mod files and their providers were not edited.

The compared native tool has SHA256
`e648760d0779a9575444b6e1938ff5ad2ce78fb1885947b029950f37e721fea7`.
The earlier tool has SHA256
`332f52deadaa5f277a9c6d4c08ed82cbc53e7e06042523e73f8309a43bb1405c`.

## Parallel fixture isolation

[observed: first workspace run and unchanged test executable] Four INI-path tests
failed in a parallel workspace run. All nine existing path tests passed when the
same compiled executable ran them serially, without a production-code change.
The fixtures were replacing and restoring the same process-wide file roots while
other fixtures were still using them.

[observed: test-helper change] Each INI fixture now holds one shared mutex until
its prior roots are restored and its temporary paths removed. Only the test helper
changes; the file confinement assertions and production file access remain intact.
The existing ScriptRunner tool smoke also passed with owned base-game data.

[observed: subsequent release workspace run] After the fixture isolation repair,
the ordinary parallel workspace suite completed: **2282 passed, 0 failed, 8 ignored**
across 136 test targets including documentation tests. No assertion, expected
value, fixture content or tolerance was relaxed to obtain this result.
[observed: final required checks] Root-workspace all-target release Clippy and
engine all-target release Clippy both passed with warnings denied. Smoke scenario
integrity verification passed for the unchanged 991-scenario catalogue.

## Limits and next work

[observed: validation scope] The real mod bodies were not executed in this corpus
pass. This removes a compile-time blocker; it does not establish functioning MIC
menus, complete Nuclear Sunset compatibility or a compatibility percentage.
Other native extension and rendering dependencies still need implementation and
runtime acceptance. The existing accepted local application has not been replaced
by this compiler-only change.

NVForever remains the engine foundation by **LootJunkieAzza and contributors**.
The original mods retain their own authorship; no original mod payload is included.
