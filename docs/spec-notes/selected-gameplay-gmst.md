# Selected gameplay GMST consumers

This native adaptation consumes actual loaded/selected GMST values. It does not
claim complete Stewie, TTW or Nuclear Sunset compatibility. No plugin version,
warning predicate, original game file or authored INI is changed.

## VATS incoming player damage

GECK VATS Settings documents fVATSPlayerDamageMult as a multiplier of all incoming
player damage while VATS is active:
https://geckwiki.com/index.php/VATS_Settings

The actual converted TTW record is Float 0.1. Selected Hardercore
SweetsSimpleSettings.ini line 85 authors 1.0 (no VATS damage reduction).
Game::damage_mult now composes the live World setting with the existing difficulty
multiplier for a player target in documented VATS modes 1 through 4. Consequently
ordinary hits, explosion limb damage and harmful-effect health damage use the same
value through their existing real consumers. Outgoing player damage and NPC versus
NPC damage retain their existing rules. Missing/non-finite/negative records retain
the previous native 1x multiplier; zero is supported. This neither replaces the
current native mark-and-fire VATS presentation nor proves original VATS parity.

## Independent Very Evil kill reward

The actual loaded YUPTTW GMST is Float 200 at global FormID 11000E3F; selected
Hardercore SweetsSimpleSettings.ini line 19 authors zero. These owned record facts
and INI provenance are in gmst-acquired-refresh-02/inventory.json.
The native engine previously used fKarmaModKillingEvilActor for both alignments.
Very Evil NPC kills now use the independent setting, with the old native Evil
reward as the no-record fallback. This is an explicit native adaptation of the
separate authored reward; the original executable implementation was not read.
Ordinary Evil, Good/Very Good, neutral murder and creature policies are unchanged.
The kill consumer reads the live setting despite the older crime settings cache,
so session SetGameSetting changes, including zero, affect the next real kill.

## Validation scope

Two focused Game integration regressions cover actual hit/health/limb application,
select/playback/off VATS, normal outgoing/NPC damage, zero, actual Hardercore INI
import, missing-record fallback, real Game::kill karma and session changes.
A first limb assertion omitted the existing player 0.5 limb multiplier; the fixture
expectation was corrected without changing production combat to fit the test.
These are behavior-level headless fixtures, not visible gameplay or an FPS benchmark.
