# Accepted work time and reserved real inventories

[design] This is a new NVForever simulation primitive, not a claim about original
game behavior. It does not alter calendar precision, merchant restock timing, or
the separate issue #1653 work.

[documented: `crates/fnv-sim/src/world.rs`, `world/accepted_time.rs`] A new World
starts with `accepted_game_hours() == Some(0.0)`. Only a positive finite interval
whose `store_calendar` call succeeded is credited: `advance_hours` and the actual
positive interval returned by `advance_to_hour`. `advance_real_seconds` delegates
to `advance_hours`, so it credits once. Direct `set_global` writes, including
same-value GameHour/GameDaysPassed writes, grant nothing. Menu/pause time has no
separate credit path; the caller must actually advance simulation time.

[design] The work counter is independent of calendar globals, so jumping a date
cannot mint production credit. Existing negative/nonfinite calendar-input rules
remain unchanged. A positive addition that overflows or cannot increase the f64
counter invalidates it permanently (`None`); consumers must fail closed. Counter
failure does not rewrite the existing calendar policy. A failed calendar store,
including a failing merchant restock, grants no accepted interval; it does not
make the existing multi-step calendar store transactional.

[documented: `Snapshot`, `World::restore`] The counter is decimal text for exact
f64 roundtrips, using the same text-preservation rationale as the fine clock.
Missing fields in older JSON snapshots migrate to valid zero. Explicit null,
malformed text, negative or nonfinite saved values are rejected on restore.
An invalid live counter saves null and cannot silently become valid on load.
Updates add a scalar only: no World snapshot or inventory clone occurs per frame.

[design] `inventory_reservations` stores only a real container FormID and a nonzero
transaction token, never another item ledger. Ordinary `inventory()` access and
`set_deleted()` reject a reserved reference with `ReservedInventory`; the escrow
owner uses its dedicated, token-checked operations. `inventory_reservation(id)`
reads the token without changing the world. Old saves default to no reservations.

[documented: `accepted_time::validate_reservations`] Restored reservations require
an existing, nondeleted dynamic reference with an inventory, explicit PLAYER_BASE
ownership, and a scriptless, nonrespawning container base. Missing refs, zero
tokens, player-reference ownership, other base kinds, deleted refs and no inventory
are rejected. This is an invariant for the escrow layer, not a security boundary
against trusted engine code that directly edits `RefState` through `get()`.

[documented: `world/accepted_time/tests.rs`] Synthetic actual-World tests cover
fractional play/wait/target-hour credit, direct writes, no-op inputs, exact save
continuation and migration, invalid numeric state, failed restock storage, and
reservation mutation/restore rejection. Test execution is delegated to the
coordinating agent; adding these tests alone is not a recorded passing run.
