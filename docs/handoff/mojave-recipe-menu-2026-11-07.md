# Native recipe menu session views — 7 November 2026

The native crafting menu now consumes runtime recipe names, category changes,
subcategory filters and renamed ingredient labels. `ShowRecipeMenu` accepts
actual temporary RCCT clones, retaining their exact session identity while
rejecting null, unknown and non-category forms. This is a bounded compatibility
repair, not completion of Nuclear Sunset.

## Why it changed

The parser and VM could enumerate recipes and execute the two original MIC
ScriptRunner files. The menu still read the initial `RecipeDef` and category
names. A second boundary rejected cloned categories because they were absent
from the parsed record map. A focused test failed on that rejection before the
one-line admission repair, then passed with the original assertions intact.

`Game::form_name` centralizes the existing VM naming lookup. `recipe_view`
returns a borrowed parsed recipe where possible and a presentation copy only
when its name/category differs. Ingredients, outputs, conditions, skills and
batch quantities are unchanged. The menu reads this live view for labels,
filters and instant crafting. Delayed production still resolves and validates
its authoritative static recipe/station binding.

## Evidence

- Release workspace: 2283 passed, 8 unchanged ignored; root and engine all-target
  release Clippy passed. The 22 focused crafting/production controls passed.
- An isolated native Game probe loaded 253 dependency-closed selected plugins.
  The unchanged `gr_p20MICingestibleloop.txt` and `gr_p20MICrecipeloop.txt` both
  compiled and ran: 59 name changes (58 ingestibles, 1 recipe), and 122 recipe
  subcategory changes. The live view matched every observed change.
- Named cloned categories: Drinks 55, Darts 18, Return 35, Poisons 14. All 122
  setters/getters, clone identities, names and unchanged recipe rules matched.
- The M4 Pro/Metal native client opened the actual MIC Drinks category. A separate owned base/DLC profile
  ran an authored presentation probe renamed one owned recipe and created category and
  subcategory clones. Its row, category label and filter were checked in the
  actual native menu. Registered expectations were applied to the runtime log.
- The run remained windowless and silent. No GPU validation errors or foreground
  activation. The captured frame verifies the menu, not presented display FPS.
- Exact local executable SHA256: `ef20596a936b1fcbe3808f170f86b0e1d14ef715036cd0d50a5f2450bdd06a98`.

The first attempted startup was correctly refused because the test executable
was beside the synthetic Data folder. It was moved outside that directory;
the install-separation guard was retained. Two command-based attempts rendered
the world but did not open the required menu and are not menu acceptance. The
final fixture uses the existing native delayed script path. Antivenom is hidden
by the selected full profile conditions, so its controlled presentation check
was performed in the separate owned base/DLC profile without changing those
conditions. No golden images,
expected labels, quantities or tolerances were loosened.

## Reproducing the focused regression

From `engine/`:

```sh
cargo test --release -p nvf-play crafting::production_aid::tests::recipe_menu_follows_script_renames_and_cloned_subcategory_without_changing_stock -- --exact
```

The unit fixture is synthetic and public. Its script mutations, real menu-call
admission, labels, filters, membership and inventory invariants require no game
assets. The private native fixture/catalogue uses owned game/mod data and is not
included in this source export; a fresh external export build remains pending.
The `NVF_PLAY_TEST_RECIPE=...:inspect` diagnostic logs the current live menu only
when explicitly requested.

## Limits and next work

The selected-plugin native run also produced unrelated missing-dependency and
embedded-script errors. Loose mod assets were not fully assembled. Those errors
were retained, not suppressed, and prevent a full Nuclear Sunset acceptance
claim. The two-script Game probe excludes ordinary quest ticking.

Temporary clones and runtime names are session state here; persistence across
save/reload was not added or accepted. Queued production through an ephemeral
category is not supported by this repair. No current-binary full playthrough,
human input/audio acceptance, hardware FPS, Long 15 gameplay or multiplayer
acceptance is claimed. Earlier base/DLC save/reload evidence belongs to its
recorded earlier executable.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later. Original mod
files remain unchanged and private. Credits and permissions remain separate.
