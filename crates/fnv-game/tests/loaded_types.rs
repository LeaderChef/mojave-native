//! Loaded record enumeration through parsed plugins and the real script VM.
//! Synthetic records only; docs/spec-notes/ext-forms.md, loaded-type enumeration.

mod common;

use common::{record, sub, tes4, top, u32s, z};
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::game::{Game, GameConfig};
use fnv_game::GameData;
use fnv_script::vm::XVal;
use fnv_script::FunctionTable;
use fnv_sim::{FormId, FormStore, RefTable, SimConfig, World};

fn named(sig: &[u8; 4], id: u32, name: &str) -> Vec<u8> {
    record(sig, 0, id, &[sub(b"EDID", z(name))])
}

fn plugins() -> Vec<Plugin> {
    // Deliberately reverse numeric IDs: enumeration must retain file order.
    let base = [
        tes4(),
        top(b"RCPE", &[named(b"RCPE", 0x920, "RecipeFirst"), named(b"RCPE", 0x910, "RecipeSecond")].concat()),
        top(b"RCCT", &named(b"RCCT", 0x930, "RecipeCategory")),
        top(b"QUST", &named(b"QUST", 0x940, "LoadedQuest")),
        top(b"FACT", &named(b"FACT", 0x950, "LoadedFaction")),
        top(b"FLST", &record(b"FLST", 0, 0x960, &[sub(b"EDID", z("ResultList")), sub(b"LNAM", u32s(&[0x950]))])),
        top(b"ALCH", &named(b"ALCH", 0x970, "InventoryControl")),
    ].concat();
    let header = [1.34f32.to_le_bytes(), 0u32.to_le_bytes(), 0x800u32.to_le_bytes()].concat();
    let patch = [
        record(b"TES4", 0, 0, &[sub(b"HEDR", header), sub(b"MAST", z("Base.esm")), sub(b"DATA", vec![0; 8])]),
        top(b"RCPE", &[
            named(b"RCPE", 0x0100_0980, "PatchRecipeFirst"),
            named(b"RCPE", 0x0100_0970, "PatchRecipeSecond"),
            named(b"RCPE", 0x920, "RecipeFirstOverride"),
        ].concat()),
    ].concat();
    vec![Plugin::parse(&base, "Base.esm".into()).unwrap(), Plugin::parse(&patch, "Patch.esp".into()).unwrap()]
}

fn query(game: &mut Game<'_>, body: &str) -> XVal {
    game.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), Default::default()).unwrap()
}

#[test]
fn loaded_types_include_recipes_and_non_inventory_records_through_the_vm() {
    let plugins = plugins();
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let functions = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &functions, &refs);
    let mut game = Game::new(&data, World::new(&forms, &refs, SimConfig::default()), GameConfig { strict: true, ..Default::default() });

    // Real RCPE parsing confirms this is enumeration, not missing recipe data.
    assert_eq!(data.recipes.len(), 4);
    let expected = [0x920, 0x910, 0x0100_0980, 0x0100_0970];
    assert_eq!(data.forms_of_type(106), expected.map(FormId));
    assert_eq!(data.edid_of[&FormId(0x920)], "RecipeFirstOverride");
    for (args, ids) in [
        ("106", expected.to_vec()),
        ("106 0", vec![0x920, 0x910]),
        ("106 1", vec![0x0100_0980, 0x0100_0970]),
        ("106 2", vec![]),
        ("107", vec![0x930]),
        ("71", vec![0x940]),
        ("8", vec![0x950]),
        ("47", vec![0x970]),
        ("-1", vec![]),
        ("65536", vec![]),
    ] {
        let source = format!("array_var a := GetLoadedTypeArray {args}\n");
        assert_eq!(query(&mut game, &format!("{source}SetFunctionValue (ar_Size a)")), XVal::Num(ids.len() as f64), "{args}");
        for (i, id) in ids.into_iter().enumerate() {
            assert_eq!(query(&mut game, &format!("{source}SetFunctionValue a[{i}]")), XVal::Ref(id), "{args}, element {i}");
        }
    }

    // The list-returning API shares the enumeration; clear, plugin filter and
    // noClear must continue to work after broadening the record types.
    query(&mut game, "GetLoadedType ResultList 106 1\nSetFunctionValue 1");
    assert_eq!(query(&mut game, "SetFunctionValue (ListGetCount ResultList)"), XVal::Num(2.0));
    assert_eq!(query(&mut game, "SetFunctionValue (ListGetNthForm ResultList 0)"), XVal::Ref(0x0100_0980));
    query(&mut game, "GetLoadedType ResultList 107 -1 1\nSetFunctionValue 1");
    assert_eq!(query(&mut game, "SetFunctionValue (ListGetCount ResultList)"), XVal::Num(3.0));
    assert_eq!(query(&mut game, "SetFunctionValue (ListGetNthForm ResultList 2)"), XVal::Ref(0x930));
}
