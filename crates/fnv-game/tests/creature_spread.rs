//! Authored records and real script dispatch; no game data.
mod common;

use common::*;
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::game::{Game, GameConfig, RunCtx};
use fnv_game::GameData;
use fnv_script::{vm::XVal, FunctionTable};
use fnv_sim::{FormId, FormStore, RefTable, SimConfig, World};

fn query(g: &mut Game<'_>, body: &str) -> XVal {
    g.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), Default::default()).unwrap()
}
fn wobble(g: &mut Game<'_>, actor: FormId) -> f64 {
    match g.run_query("Begin Function { }\nSetFunctionValue (GetCalculatedSpreadAlt 1)\nEnd\n", RunCtx { self_ref: Some(actor), ..Default::default() }).unwrap() {
        XVal::Num(n) => n,
        x => panic!("unexpected spread {x:?}"),
    }
}

#[test]
fn creature_spread_patch_is_explicit_and_unknown_patches_stay_unsupported() {
    let bytes = [tes4(),
        top(b"NPC_", &[record(b"NPC_", 0, 7, &[sub(b"EDID", z("Player"))]), record(b"NPC_", 0, 0x801, &[sub(b"EDID", z("Humanoid"))])].concat()),
        top(b"CREA", &record(b"CREA", 0, 0x802, &[sub(b"EDID", z("Creature"))])),
        top(b"WEAP", &record(b"WEAP", 0, 0x803, &[sub(b"EDID", z("Gun")), sub(b"DATA", vec![0; 15])])),
    ].concat();
    let plugins = vec![Plugin::parse(&bytes, "SpreadTest.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let functions = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &functions, &refs);
    let make = || Game::new(&data, World::new(&forms, &refs, SimConfig::default()), GameConfig::default());
    let mut g = make();
    let humanoid = g.world.place(FormId(0x801), None, [0.0; 3], [0.0; 3], 1).unwrap();
    let creature = g.world.place(FormId(0x802), None, [0.0; 3], [0.0; 3], 1).unwrap();
    for actor in [humanoid, creature, FormId::PLAYER_REF] {
        let inv = g.world.inventory_mut(actor).unwrap();
        inv.add(FormId(0x803), 1);
        inv.set_equipped(FormId(0x803));
    }
    assert_eq!(query(&mut g, r#"SetFunctionValue (GetOptionalPatch "bCreatureSpreadFix")"#), XVal::Num(0.0));
    let before = wobble(&mut g, creature);
    let human_before = wobble(&mut g, humanoid);
    let player_before = wobble(&mut g, FormId::PLAYER_REF);
    assert_eq!(query(&mut g, "SetOptionalPatch \"bCreatureSpreadFix\" 1\nSetFunctionValue (GetOptionalPatch \"bCreatureSpreadFix\")"), XVal::Num(1.0));
    assert!(g.creature_spread_exempt(creature));
    for actor in [humanoid, FormId::PLAYER_REF, FormId(0), FormId(0xDEAD)] {
        assert!(!g.creature_spread_exempt(actor));
    }
    let after = wobble(&mut g, creature);
    assert!(after < before, "actual query must consume the switch: {before} -> {after}");
    assert_eq!(wobble(&mut g, humanoid), human_before);
    assert_eq!(wobble(&mut g, FormId::PLAYER_REF), player_before);
    assert_eq!(query(&mut g, "SetOptionalPatch \"UnknownPatch\" 1\nSetFunctionValue (GetOptionalPatch \"UnknownPatch\")"), XVal::Num(0.0));
    // An invalid value or missing argument cannot silently disable an active patch.
    query(&mut g, "SetOptionalPatch \"bCreatureSpreadFix\" 2");
    query(&mut g, "SetOptionalPatch \"bCreatureSpreadFix\"");
    assert!(g.creature_spread_exempt(creature));
    let session = g.take_session();
    let mut restored = make();
    restored.put_session(session);
    assert_eq!(query(&mut restored, r#"SetFunctionValue (GetOptionalPatch "bCreatureSpreadFix")"#), XVal::Num(1.0));
    assert_eq!(query(&mut restored, "SetOptionalPatch \"bCreatureSpreadFix\" 0\nSetFunctionValue (GetOptionalPatch \"bCreatureSpreadFix\")"), XVal::Num(0.0));
    // Same original creature ref, disable reverses its real spread query exactly.
    query(&mut g, "SetOptionalPatch \"bCreatureSpreadFix\" 0");
    assert_eq!(wobble(&mut g, creature), before);
    assert_eq!(query(&mut make(), r#"SetFunctionValue (GetOptionalPatch "bCreatureSpreadFix")"#), XVal::Num(0.0));
    // Read the configured switch on a cold Game, and let a script override it.
    let root = std::env::temp_dir().join(format!("nvf-creature-spread-{}", std::process::id()));
    let dir = root.join("nvse/plugins");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("jip_nvse.ini"), "[GamePatches]\nbCreatureSpreadFix=1 ; authored config\n").unwrap();
    fnv_game::mod_files::set_roots(fnv_game::mod_files::Roots { read: vec![root.clone()], ..Default::default() });
    let mut configured = make();
    assert_eq!(query(&mut configured, r#"SetFunctionValue (GetOptionalPatch "bCreatureSpreadFix")"#), XVal::Num(1.0));
    query(&mut configured, "SetOptionalPatch \"bCreatureSpreadFix\" 0");
    let session = configured.take_session();
    let mut loaded = make();
    loaded.put_session(session);
    assert_eq!(query(&mut loaded, r#"SetFunctionValue (GetOptionalPatch "bCreatureSpreadFix")"#), XVal::Num(0.0));
    fnv_game::mod_files::set_roots(Default::default());
    std::fs::remove_dir_all(root).unwrap();
}
