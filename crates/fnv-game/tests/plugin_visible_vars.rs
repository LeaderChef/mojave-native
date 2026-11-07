//! Earlier helpers retain their authored remote slots; later runtime overrides win.
mod common;

use common::*;
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::{game::{Game, GameConfig}, GameData};
use fnv_script::{vm::XVal, FunctionTable};
use fnv_sim::{FormId, FormStore, RefTable, SimConfig, World};

const QUEST: u32 = 0x900;
const TARGET: u32 = 0x901;
const HELPER: u32 = 0x902;
const INVALID: u32 = 0x903;

fn header(masters: &[&str]) -> Vec<u8> {
    let mut subs = vec![sub(b"HEDR", [1.34f32.to_le_bytes(), 0u32.to_le_bytes(), 0x800u32.to_le_bytes()].concat())];
    for master in masters {
        subs.extend([sub(b"MAST", z(master)), sub(b"DATA", vec![0; 8])]);
    }
    record(b"TES4", 1, 0, &subs)
}

fn helper(id: u32, name: &str, field: &str) -> Vec<u8> {
    // Unknown original opcode forces the production Auto fallback path.
    let bytes = script(&[(0, None, st(0x7FFF, &[]))]);
    let mut subs = vec![sub(b"EDID", z(name))];
    subs.extend(unit(0, &bytes, &[], &[QUEST]));
    subs.push(sub(b"SCTX", format!("scn {name}\nBegin Function {{ }}\nset Q.{field} to 7\nEnd\n").into_bytes()));
    record(b"SCPT", 0, id, &subs)
}

fn plugins() -> Vec<Plugin> {
    let target = |field: &str| {
        let mut subs = vec![sub(b"EDID", z("TargetScript"))];
        subs.extend(unit(1, &script(&[]), &[(1, "Untouched", 1), (2, field, 1)], &[]));
        record(b"SCPT", 0, TARGET, &subs)
    };
    let base = [
        header(&[]),
        top(b"QUST", &record(b"QUST", 0, QUEST, &[sub(b"EDID", z("Q")), sub(b"SCRI", TARGET.to_le_bytes().to_vec())])),
        top(b"SCPT", &[target("OldName"), helper(HELPER, "EarlierHelper", "OldName"), helper(INVALID, "FutureHelper", "FutureName")].concat()),
    ].concat();
    let later = [header(&["Base.esm"]), top(b"SCPT", &target("FutureName"))].concat();
    vec![Plugin::parse(&base, "Base.esm".into()).unwrap(), Plugin::parse(&later, "Later.esp".into()).unwrap()]
}

#[test]
fn source_fallback_uses_original_names_but_executes_only_winning_scripts() {
    let plugins = plugins();
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let catalogue = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &catalogue, &refs);
    assert!(
        !fnv_game::source_compile::calls_unknown(&data.script(FormId(HELPER)).unwrap().program, &data.table),
        "the earlier helper itself must compile, not a different future-name helper: {:?}",
        data.issues
    );
    assert_eq!(data.issues.count("script_compiled_from_source"), 1, "only the earlier helper has visible names: {:?}", data.issues);
    assert_eq!(data.issues.count("script_source_does_not_compile"), 1, "a future-only name remains an error");
    let target = data.script(FormId(TARGET)).unwrap();
    assert_eq!(target.unit.plugin, "Later.esp");
    assert_eq!(target.unit.variables[1].name, "FutureName");
    assert_eq!(target.unit.variables[1].index, 2);
    assert_eq!(data.units.iter().filter(|u| u.unit.owner.form() == TARGET).count(), 1, "superseded code never becomes executable");
    let caller = data.script(FormId(HELPER)).unwrap();
    assert_eq!(caller.unit.plugin, "Base.esm");
    assert_eq!(caller.raw, 0);
    let mut game = Game::new(&data, World::new(&forms, &refs, SimConfig::default()), GameConfig::default());
    game.run_query("Begin Function { }\nCall EarlierHelper\nEnd\n", Default::default()).unwrap();
    assert_eq!(game.run_query("Begin Function { }\nSetFunctionValue Q.FutureName\nEnd\n", Default::default()).unwrap(), XVal::Num(7.0));
    assert_eq!(game.run_query("Begin Function { }\nSetFunctionValue Q.Untouched\nEnd\n", Default::default()).unwrap(), XVal::Num(0.0));
}
