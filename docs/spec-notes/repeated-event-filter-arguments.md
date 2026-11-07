# Repeated event-filter arguments

[documented] `SetEventHandlerAlt` accepts an event name, a handler, up to 16
`index::value` filter arguments and a separate `"priority"::number` argument.
Priority may occur before or after filters. Indices 0–15 cover the calling
reference and the dispatched arguments. An array is a permitted filter value.
Source: [GECK SetEventHandlerAlt](https://geckwiki.com/index.php?title=SetEventHandlerAlt).

[documented] `SetEventHandler` has two optional filter pairs and an optional
priority pair. Source: [GECK SetEventHandler](https://geckwiki.com/index.php?title=SetEventHandler).
The priority system is described in
[GECK Event Handler Priority System](https://geckwiki.com/index.php?title=Event_Handler_Priority_System).

[observed] The local documented function catalogue represents each repeated
filter group as one parameter slot. Its repeat count is in the **name**:
`filters(up-to-16)` or `filters(up to 2)`, while the type is just `pair`.
Other functions carry counts in their **type**, such as `arg(up to 15)`.
Count a repeated slot once, using its type or, if that contains no repeat count,
its name. Accept the two documented spellings. The resulting maximum arities
are 19 for SetEventHandlerAlt and 5 for SetEventHandler.

[observed] The prior compiler considered only repeat counts in parameter types,
therefore stopped SetEventHandlerAlt after four arguments. Two verified private
selected mod inputs exposed this at their third filter or final priority:

- Directive 2221, `ln_BarrelCooker.txt`, SHA256
  `e9cef0530377e9d941c29d07d54603d08e089acfa144dac3d6d234f5ecbadf11`.
- Directive 21326, `ln_GamblingLuckRebalance.txt`, SHA256
  `02b2edbc2ffad6823c7b113bd476654e4e0ffef79308237b31db27ada2e05f61`.

[observed] Provenance and exact provider hashes were checked against the private
full-selected-02 census inputs. Their source text is not included here. No mod
script or catalogue row is rewritten. No extender implementation was consulted.

[documented] Each filter remains one `XOp::Pair` expression. Retain the existing
NVSE-aware expression grammar, operator precedence and VM event path. Increasing
the documented argument bound must not turn a pair into separate arguments or
discard a final filter/priority. Compile/decode tests assert that shape, then
execute the actual VM and inspect the values delivered to its event-host hook.
This does not establish full runtime compatibility of the motivating mods.
