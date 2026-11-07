# TTW child-world radius radio

[documented] TTW NVSE's author lists `TTW_EnableRadioFix` as a toggle for radios
working in child worldspaces:
https://www.nexusmods.com/newvegas/mods/77415?tab=description

[documented] GECK Reference, Broadcast Range Type, says Radius radio fails in child
worldspaces in New Vegas, while working in their interiors. Radius, static percentage
and exterior-position reference retain their ordinary meanings:
https://geckwiki.com/index.php?title=Reference

[documented] World Spaces identifies Megaton as a child of Wasteland; WNAM is the
parent relation. PNAM shares land, LOD, map, water, climate and image-space data; it
does not contain a radio permission. World-map offsets position map markers, rather
than transforming in-world distance:
https://geckwiki.com/index.php/World_Spaces
`schemas/fnv/world.json`, WRLD WNAM/PNAM.

[observed] The owner's TTWGenericHandlerScript calls `TTW_EnableRadioFix 1` in the
DC branch and `0` in its other branch. Calls are session setup under GetGameRestarted.
The private script extraction is not shipped. A function-table name and one required
0/1 argument are justified by those calls; no numeric command opcode or plugin-version
claim is inferred.

[design: native compatibility policy] Default off preserves native `reception`
unchanged. When on, a Radius station that the original reception rejects may be
received through a listener's ancestor worldspace, using the same world positions,
radius and static formula. The station's exterior world must be on the listener's
validated parent chain. Descendants, siblings and unrelated worlds are not merged.
Everywhere, Worldspace, Linked Interiors and Current Cell ranges are unchanged.
Station enable/disable remains an earlier gate. Interior listeners still use their
existing last-outdoor position. This targets the documented Radius defect, not an
unverified blanket change to every radio range.

[design] Validate complete WNAM chains once when radio placements load. A missing
world or any cycle invalidates the extra reception path; ordinary same-world and
Everywhere reception remain available. PNAM flags are retained by the world index
and are not repurposed as radio flags. The ancestry lookup allocates only at load,
not on every station/frame. The toggle lives with other session radio state and is
not added to save data; new runtime starts off until its script sets it. Same-run
restore carries the toggle through Game::Session, alongside GetGameRestarted, so a
once-per-session initializer does not silently lose its effect on load.

[validation] One actual Game/VM call-to-engine-reception regression covers default,
on/off, nested ancestry, static-band distance, disabled station, unrelated/sibling
worlds, malformed ancestry, all other range controls, invalid arguments and actual
snapshot/cold restore plus same-run session handoff. No game
or audio device is started. This is not full TTW NVSE, optional-patch, Rock-It or
audible playback acceptance; `GetPluginVersion "ttw_nvse"` remains missing.
