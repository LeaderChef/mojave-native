# INI paths inside Data (2026-11-06)

- [documented: GECK wiki GetINIFloat/GetINIString, existing
  `jip-math-utility.md` §3] An INI file argument is relative to `Data/Config`;
  omitting it selects the calling plugin's name, with `.ini` added when absent.
- [observed: private NS Hardercore.esp, NSHardercoreSetupScript source] A setup
  script supplies `../nvse/plugins/tweaks/inis/hardercore.ini`. Combining that
  with Config names a file inside Data. The previous plain-name guard rejected
  this path. No third-party implementation code was consulted.
- [decision] Normalize dot and parent components only for INI API file arguments,
  starting from Config. Reject a parent step above Data, absolute/UNC paths,
  drives, streams and invalid file components. Generic file APIs retain their
  existing plain-name-only guard.
- [decision] Existing INI components match case without regard to case. Resolve
  symlinks and the longest existing ancestor for a new target; reject a target
  outside its explicit read/write root and dangling symlinks. This is a path
  containment check, not a claim of protection against hostile concurrent
  filesystem renames between validation and I/O.
- [decision] INI writes go only to the engine overlay, never the game's install.
  A failed disk write must not become a successful cached change. Memory-only
  sessions retain their existing in-memory behavior.
- [observed: `mod_files::path_tests`, `tests/pp.rs::ini_apis_resolve_within_data_and_write_only_the_engine_layer`]
  The regression fixture uses the actual authored numeric setting
  `bNoPipBoyInCombat=1`, with no game asset or original UI string. It verifies
  the cached aliases and regular API dispatch, overlay-only writes, case
  matching, original-data preservation and rejection of escaped paths/links.

This change does not load a mod profile, import GameSettings INIs, implement
Stewie's engine patches or establish complete Hardercore compatibility.
