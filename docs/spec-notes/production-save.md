# Production orders: save boundary

[design] A production ledger is part of `GameSnapshot`, beside its simulation. The
normal save snapshot and atomic `fnv-save` write own both; a side file or an optional
engine `SaveHook` cannot silently discard financial rights.

[observed: `engine/nvf-play/src/save.rs`, `production_before_remap` and
`read_for_load`] New saves use `nvf.save.body/2` with `fnv.game.state/2` and an
explicit typed production book. The raw gate checks its schema/shape before the
generic plugin remapper can drop map or tuple entries. Legacy `/1` body and game
snapshots are accepted only with a pristine book (absence means a pristine book).
Mixed formats, missing new books and malformed/future books are errors. A previous
reader's exact `/1` body check rejects `/2`, rather than reading and later writing
away unknown orders or command receipts. The physical container and engine state
formats are unchanged; this changes the body semantics, not its byte encoding.

[design] Any history in the initial ledger schema requires the saved plugins to
remain an identical prefix, comparing names case-insensitively and sizes exactly.
Plugins may be appended. Reordering, removing or resizing that prefix is refused
before generic FormID remapping. This intentionally protects completed receipts as
well as working orders; the format is not downgraded after a claim. A pristine
book retains the ordinary remapping path. Names and sizes alone do not establish
unchanged recipe content: the game layer also audits the saved recipe/binding
against the current data and the actual restored inventory.

[observed: `engine/nvf-play/src/script.rs`, `Runtime::restore_save`] Restoration
first builds a candidate `Game` around the actual restored `World`, so game-layer
ledger validation sees the saved stock, not an empty between-turn placeholder.
The candidate also receives a read-only snapshot of the current world session
before validation, including base-container edits carried by a same-run load.
Only after all fallible validation succeeds are the old game and world sessions
moved and both authoritative states committed. A rejected candidate leaves the
current game/world and runtime flags unchanged.

[design; observed implementation: `run_scripts`] Semantic restoration failure
records an explicit error and requests an error exit before Restart, new-game or
ordinary gameplay scripts can run. It no longer continues from the plugins with
an empty financial book. Early raw-reader errors still use the existing CLI load
failure path; neither path rewrites the rejected save.

[observed: synthetic tests in `save/production_tests.rs` and
`script/restore_tests.rs`] These call the real body reader and runtime boundary;
they use no shipped data. The tests cover legacy/no-job reading, required/malformed
books, incompatible formats, pre-remap rejection, the plugin-prefix policy and
candidate failure/success. Test execution and native financial save/claim evidence
must be recorded separately; code inspection alone is not native acceptance.
