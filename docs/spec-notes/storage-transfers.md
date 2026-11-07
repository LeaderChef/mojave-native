# Exact transfers for finite storage

[design: J6, 2026-11-06] `World::transfer_inventory_exact` and its selected-row
`_at` variant move the complete requested batch between two real inventories or
change neither. They support future production reservations and settlement stock
without introducing a separate spendable stock book. They do not implement a job,
NPC worker, elapsed-time scheduler, settlement or multiplayer authority.

[observed: local pre-change menu regressions] Ordinary storage Take All transferred
earlier rows even when a later row no longer held the displayed amount. Storing
or taking into a destination at its signed item-count limit consumed the source
while the destination saturated. The three actual `MenuModel::apply` regressions
record these outcomes before the fix; this is not original-game parity evidence.

[design] The exact primitive validates positive quantities and checked request
totals, copies only the two inventories, and uses actual `take_at` groups in caller
order. Overlapping picks cannot spend the same unit twice. Condition, ownership
and weapon modifications are retained; moved units are unequipped as in ordinary
transfer. All destination item totals must fit before commit. A refusal restores
initial materialization, pending death loot, RNG and presence state. Both holders
are validated even for an empty successful batch. This generic operation grants
no payment, permissions, script event, stat or reward.

[design] The container menu uses the exact operation for normal take/store rows
and inventory-only Take All. Successful take events, close requests and status
follow the committed transfer. A refusal leaves the menu open and emits no take.
Pickpocket chance and failure policy remain outside this primitive. A corpse's
dropped weapon is a separate physical reference picked through Activate; mixed
Take All involving those references retains the existing separate path and is
not claimed as an atomic inventory-and-reference transaction by this change.
Ordinary script Transfer/RemoveAllItems semantics and barter are unchanged.

[design] The optional take-only native test aid stops after the actual menu's
Take All, then closes it. It seeds no inventory and overrides no transfer result;
the scenario's explicit setup and real saves supply the before/after evidence.
No canonical expectations or comparators are weakened.
