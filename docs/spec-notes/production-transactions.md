# Exact inventory production

[design: J6, 2026-11-06] A production batch is an inventory exchange: all inputs
are available and consumed, all resolved outputs fit and are delivered, then it
commits. Rejection keeps the inventory and production RNG unchanged. This is an
explicit transactional design, not a claim of captured original-game parity.

[observed: local `engine/nvf-play/src/crafting.rs::make_recipe` before this change]
The previous menu checked each ingredient row independently, ignored the actual
quantity returned by `remove_item`, then added output rows in sequence. Repeated
ingredient rows could reuse the same observed count; a later invalid output could
fail after earlier consumption/delivery. Quantities used unchecked multiplication
and a signed cast. Actual menu regressions retain this before/after evidence.

[design] `World::exchange_inventory` stages one inventory and a cloned RNG. It
combines input demand with checked signed counts, requires exact consumption,
resolves outputs through the existing leveled-list resolver in original row order,
validates the leaf item types and complete destination capacities, then commits
both. No whole-world clone, separate wallet, stock database or frame-wide scan.
Failed first materialisation restores the reference, presence serial, RNG and its
pending death-item obligation. Existing metadata on unconsumed item groups stays.

[design] Leveled outputs are not aggregated into one random choice: two rows keep
two independent rolls. ChanceNone remains a legitimate zero yield. Production uses a checked resolver:
exceeding the existing per-list roll limit or overflowing a nested multiplier
refuses the exchange rather than silently reducing its yield. A production output
row has a 65,536-unit work budget [design]: recursive visits, random rolls and
entry scans spend it, including work producing no child or item. Entry scans are
charged before allocating their filtered vectors, also refusing the entire
exchange on exhaustion. The ordinary AddItem resolver retains its existing policy.
Inputs do not consume the outputs of the same batch to fund themselves.

[design] The player menu rechecks its real visibility conditions and skill at commit, and retains its existing
success-only crafted statistic, challenges, UI sound and corner messages. Inventory
observation continues to derive events from the committed real delta; it is not
reset to conceal that delta. The generic core primitive grants no player reward
and implements no station permissions of its own, making it usable by future NPC
production only after that system supplies its own authorisation and scheduling.

[design] No autonomous factory, economic time scheduler, settlement population,
caravan or multiplayer protocol is implemented by this change. Existing vanilla
merchant restocking and clock policies remain unchanged.
