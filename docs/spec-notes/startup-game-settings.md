# Startup GameSettings baseline

## Evidence and scope

- **Observed:** materialized private Nuclear Sunset winning configurations use
  `[GameSettings]` in `nvse/plugins/nvse_stewie_tweaks.ini` and direct INI children of
  `nvse/plugins/tweaks/GameSettings`. Only the real VFS winner is eligible; the
  reference profile plan is not an installed or mounted modlist.
- **Observed in this engine:** `FormStore` and `CombatForms` independently decode
  winning GMST records. `Rules::from_forms` caches combat values at startup.
  `World::change_setting` is a session overlay and snapshots omit that overlay.
- **Unknown:** relative ordering of distinct configuration files and types of
  settings supplied only by original engine defaults. Neither is inferred here.

## Candidate policy

At fallible preflight, before app/window creation, inspect actual winning loose
VFS files at those paths. Read each once, bound input size, require UTF-8 (an initial
BOM is accepted), and reject canonical paths outside the loaded Data root. Archive
entries and nested directory files are excluded. This does not add MO2 multi-root
mounting or retry a hidden lower-priority provider. Concurrent hostile filesystem
renames are outside this local startup contract.

Stage case-insensitive keys from `[GameSettings]`, ignoring other sections and
whole-line `;`/`#` comments. Numeric values must occupy the whole value: no inline
comment or quote interpretation is assumed. Integers require exact signed i32
integer literals; `1.5` and overflow fail rather than truncate. Floats must remain
finite after f32 conversion. Strings retain the trimmed literal value. Unknown
GMST keys outside the three evidenced spread and six presentation defaults and raw/unsupported
loaded types fail with source and one-based line. No prefix-based default type is
inferred. A loaded record's exact type overrides built-in metadata.

Conflicting assignments to one setting fail with both origins, within a file or
across files. Equal typed assignments are accepted. Sorted file inventory makes
diagnostics reproducible and does not claim alphabetical precedence. No partially
validated baseline or world is published. Once validated, one immutable typed
baseline supplies both simulation forms before World construction and combat
forms before cached Rules construction. No candidate files means the previous
construction path. Only play mode constructs this gameplay state.

The baseline is not a session override or a save field. Same-run loads retain the
existing session overlay above the baseline; cold loads use the current profile
baseline. Changing a GMST dynamically still does not refresh cached Combat rules;
that is a separate open limitation.

## Not full Nuclear Sunset compatibility

The original preparation report had seven unavailable selected reference GMST
files; acquisition and materialization are tracked separately. This candidate
does not replace a missing winner with an available lower-priority copy.
Four Hardercore consumer differences remain unresolved: very-evil kill karma,
VATS player damage, combat dismember chance, merchant schedule.
In particular `iMerchantRespawnDay2=1.5` is not silently treated as integer 1.
Full mod mounting, engine-default type evidence, omitted consumers, and dynamic
combat cache synchronization require separate work. No runtime/performance or
complete-profile acceptance is asserted by these unit tests.

## Narrow evidenced consumer extension

- **Observed in independent native code:** `Rules` has explicit f32 fields and a
  typed `(&str,f32)` default table for `fCrippledArms1HSpreadPenalty=0.4`,
  `fCrippledArms2HSpreadPenalty=0.6`, and `fRunningSpreadPenalty=0.2`.
  `fnv-combat::tuning::BUILTIN_SPREAD_SETTINGS` is now the shared source of these
  same values. They supplement only a nonempty startup configuration; real GMST
  records overlay their value and type. No files means the earlier map/path.
- **Observed static input inventory:** these three GMST records are absent from
  the supplied 18 real TTW masters with external YUP 13.9.1. The 43 available INI
  assignments therefore have ten remaining missing loaded types after admitting
  these three names. The source-only `iSkillPointsTagSkillMult` integer-output
  contract is not admitted as original GMST storage type evidence.
- **Observed in source:** the common headless damage formula, the tool simulation
  and native play use `CombatTuning.dt_floor`; native explosion damage and armour
  dialogue also read this same cached field. Its existing default is0.2.
- **Documented:** [GECK FMinDamMultiplier](https://geckwiki.com/index.php/FMinDamMultiplier)
  identifies this as the fraction surviving damage threshold, gives default 0.2,
  and permits 0. The indexed primary page was checked 2026-11-06; direct retrieval
  returned 403. No extender implementation or decompilation was consulted.
- **Observed:** the supplied real FalloutNV GMST 0x00147229 has four-byte
  Float 0.2 DATA. Hardercore authors literal 0. `CombatTuning::from_forms` now takes
  the loaded finite Float exactly, including 0, retaining 0.2 for absent, non-Float
  or non-finite input. No type coercion or speculative range clamp is added.
  Both native startup and headless tool damage/simulation call this constructor
  after any validated baseline is applied. Existing damage formula and save
  formats are unchanged. Runtime changes do not refresh cached tuning yet.

The [presentation extension](presentation-game-settings.md) connects six more
settings to the native light, camera and map consumers using their existing
explicit f32 defaults. Four keys remain fatal: `fFirstPersonHandFollowMult`,
`fPainDelay`, `iSkillPointsTagSkillMult`, and `iMerchantRespawnDay2`.
The latter's authored 1.5 is preserved. A full profile does not become accepted by
selectively ignoring those keys or missing providers.
