//! The real Game/VM command feeding the engine's received() consumer, no game data
//! or audio device. One scenario includes the negative/control cases.

use super::*;
use fnv_script::vm::XVal;
use fnv_sim::{SimConfig, World};
use fnv_world::{WorldIndex, Worldspace};

#[test]
fn ttw_radio_toggle_changes_parent_radius_reception_without_widening_other_ranges() {
    let mut script = ScriptState::over_no_plugins();
    let game = &mut script.rt.as_mut().unwrap().game;
    game.cfg.strict = true; // Host call errors must surface through the actual VM.
    let sim = SimState {
        forms: game.world.forms,
        refs: game.world.placed,
        world: World::new(game.world.forms, game.world.placed, SimConfig::default()),
    };
    let mut index = WorldIndex::default();
    // Parent flags share land/map/etc, not radio permissions: both zero and all
    // documented PNAM bits must allow the same explicit WNAM ancestry.
    for (id, parent, flags) in [
        (1, None, 0), (2, Some(1), 0), (3, Some(2), 0x3f),
        (4, Some(1), 0x3f), (5, None, 0),
        (6, Some(7), 0), (7, Some(6), 0), (8, Some(9), 0),
    ] {
        index.worldspaces.insert(id, Worldspace { form_id: id, parent, parent_flags: flags, ..Default::default() });
    }
    let broadcast = Broadcast {
        range: radio::BroadcastRange::Radius, radius: 10_000.0,
        static_percent: 10.0, cell: 10, world: Some(1), exterior: Some((1, [0.0; 3])),
    };
    let mut places = RadioPlaces { radius_ancestors: radius_ancestors(&index), ..Default::default() };
    places.sites.insert(100, Site { base: 101, initially_disabled: false, broadcast: Some(broadcast.clone()) });
    let listener = Listener { world: 3, interior: None, position: [9500.0, 0.0, 0.0] };
    let hear = |places: &RadioPlaces, l: &Listener, game: &fnv_game::game::Game<'_>| {
        received(&sim, places, 100, l, game.radio.ttw_child_world_radius_fix)
    };
    assert!(!game.radio.ttw_child_world_radius_fix);
    assert_eq!(hear(&places, &listener, game), None, "default native world comparison is unchanged");
    let query = |game: &mut fnv_game::game::Game<'_>, body: &str| {
        game.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), Default::default())
    };
    assert_eq!(query(game, "TTW_EnableRadioFix 1\nSetFunctionValue 71").unwrap(), XVal::Num(71.0));
    assert_eq!(hear(&places, &listener, game), Some(0.5), "actual consumer keeps the original static-band distance");
    // Real cold restore begins with defaultoff. A same-run load carries session
    // initialization alongside GetGameRestarted, rather than serializing a save flag.
    let saved = game.snapshot();
    let world = World::restore(sim.forms, sim.refs, &saved.sim).unwrap();
    let mut restored = fnv_game::game::Game::restore(game.data, world, game.cfg, &saved).unwrap();
    assert!(!restored.radio.ttw_child_world_radius_fix);
    restored.put_session(game.take_session());
    *game = restored;
    assert_eq!(hear(&places, &listener, game), Some(0.5), "same-run restore preserves the effective switch");
    assert_eq!(hear(&places, &Listener { world: 2, ..listener }, game), Some(0.5));
    assert_eq!(hear(&places, &Listener { world: 1, ..listener }, game), Some(0.5), "ordinary same-world control");
    assert_eq!(hear(&places, &Listener { position: [10_001.0, 0.0, 0.0], ..listener }, game), None);
    assert_eq!(hear(&places, &Listener { world: 5, ..listener }, game), None, "unrelated world");

    // The real enabled() gate still runs before any parent-radio fallback.
    places.sites.get_mut(&100).unwrap().initially_disabled = true;
    assert_eq!(hear(&places, &listener, game), None);
    places.sites.get_mut(&100).unwrap().initially_disabled = false;

    // Parent radio can reach descendants, not another child's private station.
    let site = places.sites.get_mut(&100).unwrap();
    site.broadcast.as_mut().unwrap().exterior = Some((4, [0.0; 3]));
    assert_eq!(hear(&places, &listener, game), None, "no sibling leakage");
    places.sites.get_mut(&100).unwrap().broadcast = Some(broadcast.clone());
    for (world, station_world) in [(6, 7), (8, 9)] {
        places.sites.get_mut(&100).unwrap().broadcast.as_mut().unwrap().exterior = Some((station_world, [0.0; 3]));
        assert_eq!(hear(&places, &Listener { world, ..listener }, game), None, "cycle/missing ancestor grants no extra reach");
    }
    places.sites.get_mut(&100).unwrap().broadcast = Some(broadcast);
    for (range, expected) in [
        (radio::BroadcastRange::Everywhere, Some(0.0)),
        (radio::BroadcastRange::WorldspaceAndLinkedInteriors, None),
        (radio::BroadcastRange::LinkedInteriors, None),
        (radio::BroadcastRange::CurrentCell, None),
        (radio::BroadcastRange::Other(9), None),
    ] {
        places.sites.get_mut(&100).unwrap().broadcast.as_mut().unwrap().range = range;
        assert_eq!(hear(&places, &listener, game), expected, "other range behavior is unchanged");
    }
    places.sites.get_mut(&100).unwrap().broadcast.as_mut().unwrap().range = radio::BroadcastRange::Radius;
    assert!(query(game, "TTW_EnableRadioFix 2\nSetFunctionValue 71").is_err());
    assert_eq!(hear(&places, &listener, game), Some(0.5), "invalid argument must not change the switch");
    assert_eq!(query(game, "TTW_EnableRadioFix 0\nSetFunctionValue 72").unwrap(), XVal::Num(72.0));
    assert_eq!(hear(&places, &listener, game), None, "Mojave/off call restores the default comparison");
    assert_eq!(hear(&places, &Listener { world: 1, ..listener }, game), Some(0.5));
    // Supporting one command does not pretend the whole TTW NVSE DLL exists.
    assert_eq!(query(game, "SetFunctionValue (GetPluginVersion \"ttw_nvse\")").unwrap(), XVal::Num(-1.0));
}
