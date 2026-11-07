# Native creature spread — 7 November 2026

TTW requests `bCreatureSpreadFix`. The native optional-patch API now enables
one implemented combat behavior, instead of reporting success for unimplemented
patches. Resolved CREA shooters lose only their inappropriate unaimed-animation
spread term when enabled. Human NPCs, the player, other spread terms, weapon/ammo
modifiers and VATS retain their existing behavior.

The switch defaults off, reads `[GamePatches]` in `NVSE/Plugins/jip_nvse.ini`,
accepts script enable/disable and carries across same-process loads. A cold Game
reads its configuration again. No original game file or extender version claim
changes. Other TTW startup patches, Stewie and the complete TTW module remain open.

## Evidence

- The authored script regression failed before the implementation, then passed.
  It covers created actor classification, the actual calculated-spread query,
  human/player/invalid-ref controls, disable reversal, configuration and session carry.
- Spread controls preserve movement, injuries, requirements, skill, weapon/ammo
  effects and player VATS behavior.
- A packaged ARM64 client ran silently without a native window on M4 Pro/Metal.
  The same robot and minigun fired **eight harmless shots at 1.2 degrees** with the
  patch enabled and **eight at 1.7 degrees** after the console source compiler
  disabled it. These are actual projectile-consumer cone half-angles, rounded to
  0.1 degree in the existing engine log. Exit was normal, with no GPU validation
  errors or foreground activation. The bounded run took **36.51 seconds**.
- Workspace release tests: **2,299 passed, 8 unchanged ignored**. Root and engine
  all-target release lint passed. Smoke Python checks: **74 run, 4 unchanged skips**.
  The local launcher passed its 38 headless checks.

The first probe used a test-aid interface that only resolves vanilla function
names; its failure is retained privately. The next run used the existing console
source compiler. Its initial checker searched for a numeric console result even
though extended source statements log `ran`; rejudging the same unchanged log at
the completed disable statement establishes the two volleys. No physical shot
expectation, tolerance or gameplay implementation was relaxed.

## Limits

This is one actual native combat capability. It does not establish full TTW or
Nuclear Sunset, original-engine shot parity, visible interaction, hardware audio,
presented FPS, a long play session, this new binary's save/reload or multiplayer.
The previous candidate remains available. The new application is a local candidate;
no public binary release is made.

See [the contract](../spec-notes/creature-spread-optional-patch.md) and
[portable evidence](mojave-creature-spread-2026-11-07.json).
