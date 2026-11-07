# Creature unaimed-spread optional patch

[documented] JIP's `SetOptionalPatch` and `GetOptionalPatch` enable, disable and
report a named patch. `bCreatureSpreadFix` removes the unaimed spread penalty
from non-human shooters that have no aiming animations. It is optional and off
by default. Sources: [GECK API](https://geckwiki.com/index.php/SetOptionalPatch),
[query](https://geckwiki.com/index.php?title=GetOptionalPatch), and the author's
[56.30 release notes](https://www.nexusmods.com/newvegas/mods/58277).

[observed: native source] NPC combat and scripted UseWeapon build a spread
stance. The latter always marks it unaimed; neither path checks creature kind.
The shared spread formula applies the same unaimed term to every shooter.

[design] Classify resolved CREA actor bases, including created references, as
creatures. NPC_ actors (including humanoid races), the player and invalid refs
are not exempt. Do not change aiming animations to pretend creatures aim.
Pass a separate unaimed-term exemption into the shared shot-cone formula.
All movement, standing, injury, requirements, skill, weapon, ammo and settling
terms retain their existing behavior. VATS and player stances are unchanged.

[design] Enable through the named script API or `[GamePatches]` in the existing
read-only `NVSE/Plugins/jip_nvse.ini` path. Read once on Game creation; explicit
script changes win for the process session and survive same-process loads.
Cold starts read configuration again. Do not write into game Data or saves.
Unknown optional patches still report zero; no extender version claim changes.

[hypothesis] This maps the documented non-human aiming-animation bug to the
native engine's CREA classification. Original-engine shot parity and full TTW
acceptance need additional runtime evidence; this patch alone proves neither.
