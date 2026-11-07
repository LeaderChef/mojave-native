# Native Hardcore need floor

[documented] The JIP author's high-level description of `bHardcoreNeedsFix` requires negative needs not to stop or distort their accumulation: https://www.nexusmods.com/newvegas/mods/58277 . No extender implementation is used.

[observed, source] Native needs already stop ordinary food/restoration changes at zero. `shift_need`, however, floors the sum of an invalid negative baseline and elapsed accumulation, losing that first interval. `Game::restore` currently retains such a baseline until the next update.

[design] While Hardcore is active, repair each player's current Dehydration, Hunger and SleepDeprivation to zero before adding a frame, wait or sleep increment. Normalize invalid restored needs before returning the restored game. Enabling Hardcore also repairs a pre-existing invalid baseline. Keep the actor value's modifier and damage pools: repair by moving its base by the current deficit, rather than erasing unrelated modifiers. Valid positive values remain unchanged.

[design] This floor is an always-on native correctness policy. `GetOptionalPatch "bHardcoreNeedsFix"` may report one only when the consumer regressions pass; an explicit zero request warns and cannot reintroduce invalid accumulation. Unknown capabilities remain unsupported; no external extender version is claimed.

[validation plan] Use an authored plugin and actual script dispatch to introduce negative bases and modifiers; compare frame, wait and sleep progress with zero-baseline controls; round-trip real World/Game JSON snapshots; retain food/restoration floors, unrelated actor values, and Hardcore reset behavior. This does not prove a full TTW or Nuclear Sunset playthrough.
