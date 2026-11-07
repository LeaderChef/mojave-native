//! A documented alias follows the same compiler path as its canonical function.
//! No game files or extender implementation are used by these fixtures.
use fnv_script::compile::{compile, Env, FormInfo};
use fnv_script::{decode, FunctionTable};
use fnv_script::unit::VarDecl;

struct Environment;

impl Env for Environment {
    fn form(&self, name: &str) -> Option<FormInfo> {
        name.eq_ignore_ascii_case("OwnMarker").then_some(FormInfo { form: 0x0100_1000, sig: *b"REFR" })
    }
    fn info(&self, form: u32) -> Option<FormInfo> {
        (form == 0x0100_1000).then_some(FormInfo { form, sig: *b"REFR" })
    }
    fn editor_id(&self, form: u32) -> Option<String> {
        (form == 0x0100_1000).then(|| "OwnMarker".into())
    }
    fn local_form(&self, form: u32) -> Option<u32> { self.info(form).map(|f| f.form) }
    fn script_vars(&self, _: u32) -> Option<Vec<VarDecl>> { None }
}

fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap()
}

#[test]
fn move_to_marker_names_the_existing_opcode() {
    let table = table();
    for name in ["MoveTo", "MoveToMarker", "movetomarker", "MOVETOMARKER"] {
        assert_eq!(table.by_name(name), Some(0x109e), "{name}");
    }
    assert_eq!(table.by_name("MoveToMarkerUnknown"), None);
}

#[test]
fn alias_preserves_caller_parameters_and_both_compiler_grammars() {
    let table = table();
    for block in ["GameMode", "_GameMode"] {
        for caller in ["Player", "r"] {
            for args in ["OwnMarker", "OwnMarker 1", "OwnMarker -12.5 3.25 -8"] {
                let canonical = format!("ref r\nbegin {block}\n{caller}.MoveTo {args}\nend");
                let alias = canonical.replace(".MoveTo ", ".MoveToMarker ");
                let first = compile(&canonical, &table, &Environment, None);
                let second = compile(&alias, &table, &Environment, None);
                assert_eq!(first.errors().count(), 0, "{canonical}: {:?}", first.errors().collect::<Vec<_>>());
                assert_eq!(second.errors().count(), 0, "{alias}: {:?}", second.errors().collect::<Vec<_>>());
                assert_eq!(first.bytes, second.bytes, "{alias}");
                assert_eq!(first.refs, second.refs, "{alias}");
                let decoded = decode(&second.bytes, &table);
                assert_eq!(decoded.report.raw_total(), 0, "{alias}");
                assert!(decoded.report.issues.is_empty(), "{alias}");
            }
        }
    }
}
