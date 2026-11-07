//! Authored script/consumer regressions for native Hardcore need repair (#19).

mod common;

use common::*;
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::game::{Game, GameConfig, RunCtx};
use fnv_game::GameData;
use fnv_script::vm::XVal;
use fnv_script::FunctionTable;
use fnv_sim::{FormId, FormStore, RefTable, SimConfig, World};
use serde_json::Value as Json;

struct Fixture {
    data: GameData,
    forms: FormStore,
    refs: RefTable,
}

fn fixture() -> Fixture {
    let timescale = record(b"GLOB", 0, 0x901, &[sub(b"EDID", z("TimeScale")), sub(b"FNAM", vec![b'f']), sub(b"FLTV", 30.0f32.to_le_bytes().to_vec())]);
    let bytes = [tes4(), top(b"GLOB", &timescale), top(b"NPC_", &record(b"NPC_", 0, 7, &[sub(b"EDID", z("Player"))]))].concat();
    let plugins = vec![Plugin::parse(&bytes, "NeedFloor.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let j: Json = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let table = FunctionTable::load(&p).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, table, &j, &refs);
    Fixture { data, forms, refs }
}

fn game(f: &Fixture, strict: bool) -> Game<'_> {
    let world = World::new(&f.forms, &f.refs, SimConfig::default());
    Game::new(&f.data, world, GameConfig { strict, ..GameConfig::default() })
}

fn q(g: &mut Game, body: &str) -> XVal {
    g.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), RunCtx::default()).unwrap_or_else(|e| panic!("{body}: {e}"))
}

fn av(g: &mut Game, name: &str) -> f64 {
    match q(g, &format!("SetFunctionValue (player.GetAV {name})")) {
        XVal::Num(n) => n,
        other => panic!("{name}: {other:?}"),
    }
}

const NEEDS: [&str; 3] = ["Dehydration", "Hunger", "SleepDeprivation"];

fn invalidate(g: &mut Game) {
    q(g, "player.SetAV Dehydration -50\nplayer.ModAV Hunger -100\nplayer.ForceAV SleepDeprivation -12");
    for name in NEEDS {
        assert!(av(g, name) < 0.0, "fixture must introduce a real negative current {name}");
    }
}

fn assert_same_needs(g: &mut Game, control: &mut Game) {
    for name in NEEDS {
        let actual = av(g, name);
        let expected = av(control, name);
        assert!(actual >= 0.0 && (actual - expected).abs() < 1e-8, "{name}: {actual} vs zero-baseline {expected}");
    }
}

#[test]
fn invalid_baselines_keep_the_first_frame_wait_and_sleep_increment() {
    let f = fixture();
    for strict in [false, true] {
        for mode in ["frame", "wait", "sleep"] {
            let mut g = game(&f, strict);
            let mut control = game(&f, strict);
            q(&mut g, "SetHardcore 1");
            q(&mut control, "SetHardcore 1");
            invalidate(&mut g);
            q(&mut g, "player.ForceAV Karma -25");
            match mode {
                "frame" => {
                    g.frames(60);
                    control.frames(60);
                }
                _ => {
                    g.pass_hours(1.0, mode == "sleep");
                    control.pass_hours(1.0, mode == "sleep");
                }
            }
            assert_same_needs(&mut g, &mut control);
            assert!(av(&mut control, "Dehydration") > 0.0, "control must actually advance in {mode}");
            assert_eq!(av(&mut g, "Karma"), -25.0, "need repair must not clamp unrelated actor values");

            // Existing restorative/food-style floor and subsequent accumulation remain live.
            q(&mut g, "player.RestoreAV Dehydration 100000\nplayer.RestoreAV Hunger 100000\nplayer.RestoreAV SleepDeprivation 100000");
            for name in NEEDS {
                assert!(av(&mut g, name).abs() < 1e-8);
            }
            g.frames(60);
            assert!((av(&mut g, "Dehydration") - 0.1).abs() < 1e-8);
            assert!((av(&mut g, "Hunger") - 0.04).abs() < 1e-8);
            assert!((av(&mut g, "SleepDeprivation") - 0.02).abs() < 1e-8);
        }
    }
}

#[test]
fn restored_or_newly_enabled_hardcore_repairs_current_needs_without_erasing_pools() {
    let f = fixture();
    for strict in [false, true] {
        let mut g = game(&f, strict);
        q(&mut g, "SetHardcore 1");
        invalidate(&mut g);
        q(&mut g, "player.ForceAV Karma -25");
        let before = g.extra[&FormId::PLAYER_REF].clone();
        let world_save = serde_json::from_slice(&serde_json::to_vec(&g.world.snapshot()).unwrap()).unwrap();
        let game_save = serde_json::from_slice(&serde_json::to_vec(&g.snapshot()).unwrap()).unwrap();
        let world = World::restore(&f.forms, &f.refs, &world_save).unwrap();
        let mut loaded = Game::restore(&f.data, world, GameConfig { strict, ..Default::default() }, &game_save).unwrap();
        let mut control = game(&f, strict);
        q(&mut control, "SetHardcore 1");
        assert_same_needs(&mut loaded, &mut control);
        assert_eq!(loaded.extra[&FormId::PLAYER_REF].av_permanent, before.av_permanent, "repair moves the base, not permanent modifiers");
        assert_eq!(loaded.extra[&FormId::PLAYER_REF].av_damage, before.av_damage);
        assert_eq!(av(&mut loaded, "Karma"), -25.0);
        loaded.frames(60);
        control.frames(60);
        assert_same_needs(&mut loaded, &mut control);

        q(&mut loaded, "SetHardcore 0");
        for name in NEEDS {
            assert!(av(&mut loaded, name).abs() < 1e-8);
        }
        invalidate(&mut loaded);
        loaded.frames(1);
        assert!(av(&mut loaded, "Hunger") < 0.0, "Hardcore-off script values retain ordinary semantics");
        q(&mut loaded, "SetHardcore 1");
        for name in NEEDS {
            assert!(av(&mut loaded, name).abs() < 1e-8, "enabling Hardcore repairs {name} immediately");
        }
        let capability = "SetFunctionValue (GetOptionalPatch \"bHardcoreNeedsFix\")";
        assert_eq!(q(&mut loaded, capability), XVal::Num(1.0));
        q(&mut loaded, "SetOptionalPatch \"bHardcoreNeedsFix\" 0");
        assert_eq!(q(&mut loaded, capability), XVal::Num(1.0));
        assert_eq!(q(&mut loaded, "SetFunctionValue (GetOptionalPatch \"bNotImplemented\")"), XVal::Num(0.0));
        invalidate(&mut loaded);
        loaded.pass_hours(1.0, false);
        q(&mut control, "SetHardcore 0\nSetHardcore 1");
        control.pass_hours(1.0, false);
        assert_same_needs(&mut loaded, &mut control);
    }
}

#[test]
fn repair_leaves_valid_positive_needs_and_unrelated_values_unchanged() {
    let f = fixture();
    let mut g = game(&f, true);
    q(&mut g, "SetHardcore 1\nplayer.ForceAV Dehydration 250\nplayer.ForceAV Hunger 100\nplayer.ForceAV SleepDeprivation 50\nplayer.ForceAV RadLevel -50\nplayer.ForceAV Karma -25");
    let before = g.extra[&FormId::PLAYER_REF].clone();
    let world_save = serde_json::from_slice(&serde_json::to_vec(&g.world.snapshot()).unwrap()).unwrap();
    let game_save = serde_json::from_slice(&serde_json::to_vec(&g.snapshot()).unwrap()).unwrap();
    let world = World::restore(&f.forms, &f.refs, &world_save).unwrap();
    let mut loaded = Game::restore(&f.data, world, GameConfig { strict: true, ..Default::default() }, &game_save).unwrap();
    for (name, value) in [("Dehydration", 250.0), ("Hunger", 100.0), ("SleepDeprivation", 50.0), ("RadLevel", -50.0), ("Karma", -25.0)] {
        assert_eq!(av(&mut loaded, name), value);
    }
    let after = &loaded.extra[&FormId::PLAYER_REF];
    assert_eq!(after.actor_values, before.actor_values);
    assert_eq!(after.av_permanent, before.av_permanent);
    assert_eq!(after.av_temporary, before.av_temporary);
    assert_eq!(after.av_damage, before.av_damage);
}
