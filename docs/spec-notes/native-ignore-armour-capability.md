# Native ignore-armor capability

[documented] The GECK Weapons page describes WEAP's `Ignores Normal Weapon Resistance` flag as bypassing the target's armor, and identifies JIP's `bIgnoreDTDRFix` as its New Vegas fix: https://geckwiki.com/index.php/Weapons . The JIP author's public description additionally says DT is subtracted before DR is applied: https://www.nexusmods.com/newvegas/mods/58277?tab=description . This is behavioral documentation, not extender implementation material.

[observed] `fnv_combat::damage::hit` already sets DT and DR to zero for that WEAP flag. For ordinary weapons it applies weapon/ammo/perk damage modifiers before subtracting DT (with the native damage floor), then scales the remainder by DR. The playable player's melee, NPC melee and projectile consumers call this function. Native optional-patch dispatch nevertheless reports this named behavior as absent.

[observed, private owned data] A read-only 18-plugin TTW consumer found 18 flagged winning weapons, including automatic melee and unarmed records. Their damage was identical against unarmored and DT/DR-protected targets. Ordinary melee/unarmed controls retained armor reduction. The actual script query returned zero. No owned records, extracted stats or private probe payloads are distributed.

[hypothesis/design] Expose this already-implemented native behavior as an always-on `bIgnoreDTDRFix` capability, case-insensitively. An explicit zero request warns and leaves it active, as with the native clock and need-floor invariants. Do not add an unused mutable switch, alter damage formulas merely to support the query, claim an external plugin/version, or change any dependency warning. Unsupported patch names continue to return zero.

[validation plan] Authored tests must cover automatic melee/unarmed flag behavior with ordinary controls, DT-before-DR and ammo/perk ordering, real script dispatch and the always-on policy. Then run required changed-source checks, repeat only the affected owned-data consumer, and perform one isolated silent native smoke. These are bounded native checks, not full JIP patch-version, armor-wear parity, TTW, rendered combat or multiplayer acceptance.
