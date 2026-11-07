//! Actual HEDR parser -> GameData -> game/VM -> TTW_GetPlugins, with no game data.

mod common;

use common::{record, sub};
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::game::{Game, GameConfig};
use fnv_game::GameData;
use fnv_script::vm::XVal;
use fnv_script::FunctionTable;
use fnv_sim::{FormStore, RefTable, SimConfig, World};

fn query(game: &mut Game<'_>, body: &str) -> XVal {
    game.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), Default::default()).unwrap()
}

#[test]
fn ttw_plugin_rows_use_exact_loaded_hedr_versions_in_order() {
    // An older real header in the middle catches defaulting to 1.34, sorting,
    // and accidentally reporting a virtual extender-plugin version instead.
    let expected = [("First.esm", 1.34f32), ("Older.esm", 1.20f32), ("Last.esm", 1.33f32)];
    let plugins: Vec<Plugin> = expected.iter().map(|(name, version)| {
        let hedr = [version.to_le_bytes(), 0u32.to_le_bytes(), 0x800u32.to_le_bytes()].concat();
        Plugin::parse(&record(b"TES4", 1, 0, &[sub(b"HEDR", hedr)]), (*name).into()).unwrap()
    }).collect();
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let functions = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &functions, &refs);
    assert_eq!(data.plugin_versions, expected.map(|(_, v)| v));
    let mut game = Game::new(&data, World::new(&forms, &refs, SimConfig::default()), GameConfig { strict: true, ..Default::default() });
    assert_eq!(query(&mut game, "array_var a\nlet a := TTW_GetPlugins\nSetFunctionValue (ar_Size a)"), XVal::Num(3.0));
    for (i, (name, version)) in expected.into_iter().enumerate() {
        let row = format!("array_var a\narray_var r\nlet a := TTW_GetPlugins\nlet r := a[{i}]\n");
        assert_eq!(query(&mut game, &format!("{row}SetFunctionValue (ar_Size r)")), XVal::Num(3.0));
        assert_eq!(query(&mut game, &format!("{row}SetFunctionValue r[0]")), XVal::Str(name.into()));
        assert_eq!(query(&mut game, &format!("{row}SetFunctionValue r[1]")), XVal::Num(f64::from(version)));
        assert_eq!(query(&mut game, &format!("{row}SetFunctionValue r[2]")), XVal::Str(String::new()));
        assert_eq!(query(&mut game, &format!("{row}SetFunctionValue (r[1] < 1.3)")), XVal::Num(f64::from(u8::from(version < 1.3))));
    }
    // File header metadata must not invent native support for missing DLL APIs.
    for plugin in ["lStewieAl's Tweaks", "ttw_nvse"] {
        assert_eq!(query(&mut game, &format!(r#"SetFunctionValue (GetPluginVersion "{plugin}")"#)), XVal::Num(-1.0));
        assert_eq!(query(&mut game, &format!(r#"SetFunctionValue (IsPluginInstalled "{plugin}")"#)), XVal::Num(0.0));
    }
    // Native platform capability through the real VM dispatch. Do not duplicate
    // its allocator/probe implementation in a test or claim a PE file was patched.
    let capability = query(&mut game, "SetFunctionValue (GetIsLAA)");
    if usize::BITS == 64 {
        assert!(matches!(capability, XVal::Num(1.0 | 2.0)), "{capability:?}");
    } else {
        assert_eq!(capability, XVal::Num(0.0));
    }
    assert_eq!(query(&mut game, "SetFunctionValue (GetIsLAA)"), capability);
    println!("GetIsLAA native capability: {capability:?}; pointer width: {}", usize::BITS);
}
