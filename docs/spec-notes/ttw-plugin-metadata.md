# TTW loaded-plugin header metadata

- [observed: private TTW 3.4 data, TTWGenericHandler stage 0 and
  TTWGenericHandlerScript] TTW checks the version at index 1 of each
  `TTW_GetPlugins` row against 1.3 to detect plugins that need conversion.
- [observed: `crates/fnv-esm/src/plugin.rs`; `fnv-game/src/data.rs`]
  `Plugin.header.version` is read from TES4 HEDR, and `GameData::build` retains
  those versions in the same order as `GameData.load_order`.
- [design] Return each loaded name and its actual retained HEDR version in the
  existing `[name, version, description]` row. Preserve the current empty
  description. A real version below 1.3 remains below 1.3; do not replace it with
  a modern default. An inconsistent metadata length is an error, rather than a
  zero version or a truncated plugin list.
- [scope] This is plugin-file metadata, not an extender-plugin version or a
  promise that TTW dependencies work. Do not register Stewie, ROOG or ttw_nvse,
  change GetIsLAA, suppress messages or edit game data.

The focused synthetic regression parses three different HEDR versions, builds
`GameData`, and invokes the real game/VM query path to inspect all row positions
and TTW's below-1.3 predicate. No proprietary data is included in the test.
