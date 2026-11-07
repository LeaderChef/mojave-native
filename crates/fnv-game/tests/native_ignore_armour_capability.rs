//! Actual compiled script dispatch for an always-on native combat invariant.
mod common;
use common::*;
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::{game::{Game, GameConfig}, GameData};
use fnv_script::{FunctionTable, vm::XVal};
use fnv_sim::{FormStore, RefTable, SimConfig, World};

fn query(g: &mut Game<'_>, body: &str) -> XVal {
    g.run_query(&format!("Begin Function {{ }}\n{body}\nEnd"), Default::default()).unwrap()
}

#[test]
fn ignore_armor_report_matches_the_always_on_native_damage_policy() {
    let bytes = [tes4(), top(b"NPC_", &record(b"NPC_", 0, 7, &[sub(b"EDID", z("Player"))]))].concat();
    let plugins = vec![Plugin::parse(&bytes, "NativeArmor.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let catalog = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &catalog, &refs);
    let mut g = Game::new(&data, World::new(&forms, &refs, SimConfig::default()), GameConfig::default());
    for name in ["bIgnoreDTDRFix", "BIGNOREDTDRFIX", "bignoredtdrfix"] {
        assert_eq!(query(&mut g, &format!("SetFunctionValue (GetOptionalPatch \"{name}\")")), XVal::Num(1.0));
    }
    for flag in ["1", "0", "2"] {
        assert_eq!(query(&mut g, &format!("SetOptionalPatch \"bIgnoreDTDRFix\" {flag}\nSetFunctionValue (GetOptionalPatch \"bIgnoreDTDRFix\")")), XVal::Num(1.0));
    }
    assert_eq!(query(&mut g, "SetOptionalPatch \"UnknownNativePatch\" 1\nSetFunctionValue (GetOptionalPatch \"UnknownNativePatch\")"), XVal::Num(0.0));
    assert_eq!(query(&mut g, "SetFunctionValue (GetPluginVersion \"lStewieAl's Tweaks\")"), XVal::Num(-1.0));
    assert_eq!(query(&mut g, "SetFunctionValue (GetPluginVersion \"ttw_nvse\")"), XVal::Num(-1.0));
}
