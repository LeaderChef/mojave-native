# Native presentation GameSettings

## Evidence and boundaries

- **[observed]** The independent native input consumer used a typed `f32`
  flashlight hold of 1.0 seconds (`flashlight::HOLD_SECONDS`), a third-person
  distance ceiling of 450.0 game units (`control::MAX_DISTANCE`), and `f32`
  zoom bounds 1.0 to 4.0 for both Pip-Boy maps (`pipboy/data.rs::ZOOM`).
- **[documented]** The author of [Faster PipBoy Light](https://www.nexusmods.com/fallout3/mods/3253/)
  describes `fPlayerPipBoyLightTimer` as the time holding the Pip-Boy key before
  its light activates. The author of [360 Movement](https://www.nexusmods.com/newvegas/mods/71940)
  describes `fChaseCameraMax` as the limit on third-person zoom distance.
- **[documented]** [Stewie's public documentation](https://www.nexusmods.com/newvegas/mods/66347?tab=docs)
  shows world/local map minimum zoom assignments under GameSettings. The
  [GECK settings list](https://geckwiki.com/index.php?title=Complete_List_of_GameSettings)
  lists the four map zoom bounds. These documents describe user-visible options;
  no plugin implementation source was consulted.
- **[observed]** All six named records are absent from the pinned 18-plugin TTW
  inventory. A native explicit `(&str, f32)` table now names just these six
  existing consumers. Its defaults preserve the preceding native behavior.
  GECK's original-game defaults differ: this is not a vanilla parity claim and
  their absence is not filled by guessing a type from the first letter.

## Contract

**[design]** Only a nonempty startup INI set supplements the actual record map
with the six explicit native Float contracts. Actual records overlay the table,
then the existing parser stages the complete immutable baseline atomically.
An authored presentation setting must resolve to a finite Float usable by its
consumer: light time must be nonnegative, chase distance at least the existing
60-unit near limit, and each map needs positive, ordered bounds. Errors name the
INI source and line. No invalid pair is silently reordered or partially applied.

**[design]** The light input, third-person wheel, map wheel, controller zoom,
opening, panning and drawing use the actual `SimState::world` settings, including
its session layer. Each lookup is in memory; no per-frame INI or archive read.
No value is copied into a save. The existing native defaults remain for absent
or unusable runtime values; a runtime map-bound change clamps the view safely.

**[observed]** No proven existing native contract was found for the original
`fFirstPersonHandFollowMult` behavior: `view_sway.rs` explicitly calls its
interpretation a hypothesis and its constants native engineering choices. No
existing named pain-delay consumer was found. Those two settings remain fatal,
as do `iSkillPointsTagSkillMult` (original storage type unproven) and
`iMerchantRespawnDay2=1.5` (literal preserved). Registering them as ignored
settings would hide incomplete compatibility and is not done here.

**[design]** This candidate proves isolated configuration and consumer behavior
only. Acquisition and mounting of all ten winning configurations are tracked
separately; this source change does not install those archives. Full Nuclear
Sunset, native runtime, GUI, game-performance, and vanilla behavior acceptance
remain open.
