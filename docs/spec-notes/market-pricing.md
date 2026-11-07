# Optional stock-sensitive barter

[design, J6 2026-11-06] This is an opt-in part of the requested living economy,
not original-game parity or an installed community mod. `stock-prices` in the
settings page applies to the next player barter menu. Off preserves vanilla
pricing. Each vendor inventory holder is a separate market, not yet a simulated
geographic region, settlement or faction economy.

The simulation's item inventories and caps remain the sole authority. The saved
`j6_market_pricing` engine hook stores only price targets, never another balance
or item stock. First observation captures a target of at least eight units per
commodity in equivalent-quality units. Stock and cart quantities are weighted by
the exact same positive finite `value_at(base, condition)/base` ratio as prices.
The ratio is not upper-clamped separately from price: modified condition GMSTs
can make it greater than one. Broken zero-value weapons contribute zero supply;
counting them as whole stock allowed a repeatable profitable price manipulation
in a real MenuModel regression. Physical item quantities stay unchanged.
Player items absent from vendor stock are captured at zero stock too;
otherwise selling them and reopening could silently reset their target.

[design] Pressure at vendor stock `s` and target `t` is
`clamp(1.5 - s/(2*t), 0.5, 1.5)`. Buying spans the stock interval downwards,
selling upwards. The analytic integral gives the average pressure over the whole
quantity, with bounded work rather than one iteration per unit. Offered condition
stacks of the same item use their combined quantity; stack splitting cannot reset
the supply curve. Existing Barter skill, condition and discounts apply. In the
optional mode the full raw cart is totalled first, then rounded down as a net
payment to the player (a purchase rounds up in cost). Per-unit rounding made a
target8/stock11/value2 bulk purchase and ten single sales generate one free cap
in a production MenuModel regression. Default vanilla rounding is unchanged.
Rows in this mode show approximate per-unit prices with `~` and two decimals;
the central cart total is the amount that actually changes hands.
This choice is a first tuning model, not calibrated original-game behaviour.

The menu reads actual inventories when refreshing. An accept whose stock change
changed the offered quantities or total requires review instead of silently
executing a new price. Caps, quantity prompts, category/service filters, quest
items and actual transfers still use the existing barter model. The optional
transaction stages only the two touched Inventory clones using the simulation's
removal order and unequip operation, checks every exact count and cap destination
overflow, then commits under one exclusive World borrow. No live partial transfer
is made. Invalid/nonfinite or out-of-range prices are refused. Cancelling moves
no items or caps. This is not a new network transaction protocol.

[observed: source] `World::restock_merchants` can still reset Respawns containers;
NPC purchases in `fnv-ai::errand` use a separate price path. Neither is changed by
this slice. There is no finite world economy, autonomous industry, caravan,
employment, settlement production, NPC price integration or multiplayer claim.
Future financial authority needs validated typed saved state and shared atomic
transactions; optional price-target metadata alone is not that authority.

The existing engine load pipeline remaps FormId strings throughout the save body;
this hook must not remap them again. Unknown/invalid pricing metadata disables
the optional mode and warns without replacing any inventory. Old saves missing
the hook remain valid. The setting remains the user's INI choice, not a saved
mode that overrides the user's next start.

Acceptance: unchanged vanilla model tests; actual MenuModel buy/sell/cancel and
failed payment; large carts and condition-stack grouping; changed-stock review;
immutable targets across save/reload; actual native barter inventory/caps and
save roundtrip. None of these alone establishes NPC trade or a living world.
