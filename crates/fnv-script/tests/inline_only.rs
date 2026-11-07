//! Authored source -> bytes -> IR -> VM tests, without game/mod payloads.
use fnv_script::compile::{compile, Env, FormInfo, PLAYER};
use fnv_script::unit::RefSlot;
use fnv_script::vm::{ArgValue, ExtStore, Host, RunOpts, Value, Vm};
use fnv_script::{decode, FunctionTable, RefIndex, Stmt, VarDecl};

const ITEM: u32 = 0x0100_0800;
struct Forms;
impl Env for Forms {
    fn form(&self, name: &str) -> Option<FormInfo> {
        name.eq_ignore_ascii_case("ProbeItem").then_some(FormInfo { form: ITEM, sig: *b"MISC" })
    }
    fn info(&self, form: u32) -> Option<FormInfo> {
        (form == ITEM).then_some(FormInfo { form, sig: *b"MISC" })
    }
    fn editor_id(&self, form: u32) -> Option<String> { (form == ITEM).then(|| "ProbeItem".into()) }
    fn local_form(&self, local: u32) -> Option<u32> { Some(local) }
    fn script_vars(&self, _: u32) -> Option<Vec<VarDecl>> { None }
}

struct State<'a> {
    table: &'a FunctionTable,
    refs: Vec<RefSlot>,
    ext: ExtStore,
    calls: Vec<(String, Option<Value>, Vec<ArgValue>)>,
}
impl Host for State<'_> {
    fn get_var(&mut self, _: &fnv_script::VarRef) -> Result<Value, String> {
        Err("the authored fixtures declare no variables".into())
    }
    fn set_var(&mut self, _: &fnv_script::VarRef, _: Value) -> Result<(), String> {
        Err("the authored fixtures assign no variables".into())
    }
    fn ref_value(&mut self, index: RefIndex) -> Result<Value, String> {
        match self.refs.get(index.0 as usize - 1) {
            Some(RefSlot::Form(id)) => Ok(Value::Ref(*id)),
            _ => Err("only authored form references are expected".into()),
        }
    }
    fn ext(&mut self) -> Option<&mut ExtStore> { Some(&mut self.ext) }
    fn call(&mut self, opcode: u16, on: Option<Value>, args: &[ArgValue]) -> Result<Value, String> {
        let name = self.table.get(opcode).ok_or("unknown opcode")?.name.clone();
        self.calls.push((name.clone(), on, args.to_vec()));
        match name.as_str() {
            "GetActorValue" => {
                assert_eq!(on, Some(Value::Ref(PLAYER)));
                assert_eq!(args.first().and_then(ArgValue::value), Some(Value::Num(11.0)));
                Ok(Value::Num(7.0))
            }
            "AddItem" | "RestoreActorValue" => Ok(Value::Num(0.0)),
            _ => Err(format!("unexpected host fallback {name}")),
        }
    }
}

fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap()
}

#[test]
fn inline_getter_alone_selects_expression_grammar_and_preserves_receiver_and_order() {
    for text in [
        "Player.AddItem ProbeItem (Player.GetAV Luck)",
        "Player.RestoreAV Health ((Player.GetActorValue Luck) * 2)",
    ] {
        let t = table();
        let c = compile(text, &t, &Forms, None);
        assert!(c.errors().next().is_none(), "{text}: {:?}", c.diags);
        let p = decode(&c.bytes, &t);
        assert_eq!(p.report.raw_total(), 0);
        assert!(matches!(&p.program.top[..], [Stmt::Eval { .. }]));
        let mut state = State { table: &t, refs: c.refs, ext: ExtStore::default(), calls: Vec::new() };
        Vm::new(&t, &mut state, RunOpts { strict: true, ..RunOpts::default() }).run(&p.program.top).unwrap();
        assert_eq!(state.calls.len(), 2);
        assert_eq!(state.calls[0].0, "GetActorValue", "inner getter runs first");
        assert_eq!(state.calls[1].1, Some(Value::Ref(PLAYER)));
        if state.calls[1].0 == "AddItem" {
            assert_eq!(state.calls[1].2[0].value(), Some(Value::Ref(ITEM)));
            assert_eq!(state.calls[1].2[1].value(), Some(Value::Num(7.0)));
        } else {
            assert_eq!(state.calls[1].0, "RestoreActorValue");
            assert_eq!(state.calls[1].2[0].value(), Some(Value::Num(16.0)));
            assert_eq!(state.calls[1].2[1].value(), Some(Value::Num(14.0)));
        }
    }
}

#[test]
fn inline_arithmetic_needs_no_unrelated_let_or_initialized_variable() {
    let t = table();
    let c = compile("Player.AddItem ProbeItem (1 + 3 * 2)", &t, &Forms, None);
    assert!(c.errors().next().is_none(), "{:?}", c.diags);
    let p = decode(&c.bytes, &t);
    assert_eq!(p.report.raw_total(), 0);
    assert!(matches!(&p.program.top[..], [Stmt::Eval { .. }]));
    let mut state = State { table: &t, refs: c.refs, ext: ExtStore::default(), calls: Vec::new() };
    Vm::new(&t, &mut state, RunOpts { strict: true, ..RunOpts::default() }).run(&p.program.top).unwrap();
    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].2[1].value(), Some(Value::Num(7.0)));
}

#[test]
fn plain_grouped_form_and_signed_number_keep_the_classic_call_path() {
    let t = table();
    let c = compile("Player.AddItem (ProbeItem) (-3)", &t, &Forms, None);
    assert!(c.errors().next().is_none(), "{:?}", c.diags);
    let p = decode(&c.bytes, &t);
    assert_eq!(p.report.raw_total(), 0);
    assert!(matches!(&p.program.top[..], [Stmt::Call { .. }]));
    let mut state = State { table: &t, refs: c.refs, ext: ExtStore::default(), calls: Vec::new() };
    Vm::new(&t, &mut state, RunOpts { strict: true, ..RunOpts::default() }).run(&p.program.top).unwrap();
    assert_eq!(state.calls[0].2[1].value(), Some(Value::Num(-3.0)));
}

#[test]
fn inline_syntax_does_not_hide_unknown_receivers_or_missing_operands() {
    for text in [
        "Player.AddItem ProbeItem (MissingRef.GetAV Luck)",
        "Player.AddItem ProbeItem (1 +)",
        "Player.AddItem ProbeItem ((Player.GetAV Luck)",
    ] {
        let c = compile(text, &table(), &Forms, None);
        assert!(c.errors().next().is_some(), "must reject {text}");
    }
}
