# Loaded recipe records — 7 November 2026

## Problem and fix

[observed: local ARM64/Metal run] A newly compiled grouped-foreach client correctly
iterated arrays and strings and found 226 ingestibles, but enumerated zero recipes.
The generic runtime health check passed; that result did not establish a working
recipe list. This failure was retained rather than accepted as mod compatibility.

[documented] The [GECK Form Type IDs table](https://geckwiki.com/index.php/Form_Type_IDs)
assigns 106 to `RCPE`, 107 to `RCCT`, 71 to quests and 8 to factions. The engine
already has that complete documented table in its script VM.

[observed: code] The game's shared loaded-form enumeration and first-definition
ordering both used a different, inventory-only lookup. The recipe parser was
already loading the actual recipe records. Both lookups now reuse the VM's full
form-type table. Inventory mutation/classification helpers are unchanged. Invalid
codes cannot wrap into valid ones. Record overrides remain a single identity in
the first defining plugin's file order.

[hypothesis] Original-engine order for non-inventory forms has not been measured;
the existing ordering model is reused. This does not claim an original-game parity
capture, implemented Windows DLL loading or a completed MIC crafting interface.

## Regression and owned-data evidence

[observed: synthetic plugins through the real parser and VM] Before the production
change, the new regression failed because four parsed recipes enumerated as an
empty list. After the fix, it passed. The fixture covers recipes, categories,
quests, factions, an inventory control, both loaded-type APIs, plugin filtering,
override deduplication, non-numeric file order, list clearing/appending and invalid
codes. No original mod code, game record or expected value was altered.

[observed: independent, read-only before-fix tool] A header-only scan of the owned base game plus nine official add-ons, remapping
each record through its declared masters and counting unique global identities,
establishes 291 recipes, 11 categories, 640 quests and 226 ingestibles. These counts
are runtime expectations, not counts invented by the changed function. The first
aggregate estimate subtracted all earlier-master-index records as overrides; that
incorrectly lost five first definitions injected by the four item packs. The
original estimate is retained privately, and the independent header scan handles
those identities without reading the engine log or using its type lookup.

## Native validation and current candidate

[observed: M4 Pro, Metal, owned base + nine add-ons] One silent/windowless run
completed in 36.87 seconds; a fresh-process save reload completed in 17.11 seconds.
Both exited normally, with zero GPU validation errors and no window activation.
Both script executions found all 291 recipes, 11 categories, 640 quests and
226 ingestibles; base-only and first-DLC filters returned 105 and 77 recipes.
The registered `loaded-type-records` smoke expectations also passed against the
first native log. No comparator or expectation was loosened.

[observed: private output inspection] The run mixed 28.076 seconds of finite audio,
peak 0.721925, with no samples beyond full scale. Checked player inventory, actor
values/pools, camera mode, magazine and position roundtripped in the fresh save.
Both save thumbnails were nonblank 512×288 images. No standalone screenshots were
captured. The Mac was locked; no human input, physical speaker playback, presented
FPS or long-session claim follows from these runs.

[observed: build checks] Release workspace tests: **2283 passed, 0 failed, 8
unchanged ignored**, over 137 targets including docs. Root-workspace and engine
all-target release Clippy passed with warnings denied. The private development catalogue (992 scenarios)
passes integrity verification. Its new loaded-type scenario expectations were
applied unchanged to the actual native log; this source snapshot does not bundle
the full private smoke-runner tree. The separate candidate launcher passed 35 headless
checks, sees all nine official add-ons, and pins its exact native engine hash.

Current native engine SHA256:
`eacf11bd5ba34b5d4fc784a6bb8b766854fade60b513433afe8a1c224a3d40b7`.
It is a Mach-O 64-bit ARM64 release executable. The earlier zero-recipe build and
all previously delivered applications/profiles remain preserved. The new local
candidate uses its own Candidate-20261107-03 profile; no public binary release is
being announced. No original mod body was executed here.

The grouped-foreach compiler correction is included in this client as well.
Remaining acceptance includes actual MIC interfaces, full Nuclear Sunset/TTW,
visible first/third-person gameplay, measured performance, long sessions and
cross-platform multiplayer. The source export still needs independent build
reproduction.

NVForever is the Rust engine foundation by **LootJunkieAzza and contributors**.
Original mod authors retain their own credit. No game or mod payload is included.
