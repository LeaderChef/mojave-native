//! Runtime enum arguments in documented compiler-override blocks.
use std::collections::BTreeMap;
use fnv_script::compile::{compile, Env, FormInfo, PLAYER};
use fnv_script::unit::RefSlot;
use fnv_script::vm::{ArgValue, ExtStore, Host, RunOpts, Value, Vm};
use fnv_script::{decode, FunctionTable, RefIndex, VarDecl, VarRef, VarSlot};

struct Forms;
impl Env for Forms {
    fn form(&self, _: &str) -> Option<FormInfo> { None }
    fn info(&self, _: u32) -> Option<FormInfo> { None }
    fn editor_id(&self, _: u32) -> Option<String> { None }
    fn local_form(&self, local: u32) -> Option<u32> { Some(local) }
    fn script_vars(&self, _: u32) -> Option<Vec<VarDecl>> { None }
}

#[derive(Default)]
struct State {
    vars: BTreeMap<u16, Value>,
    refs: Vec<RefSlot>,
    codes: Vec<f64>,
    ext: ExtStore,
}
impl Host for State {
    fn get_var(&mut self, var: &VarRef) -> Result<Value, String> {
        let VarSlot::Local { index, .. } = var.var else { return Err("unexpected global".into()) };
        Ok(self.vars.get(&index).copied().unwrap_or_default())
    }
    fn set_var(&mut self, var: &VarRef, value: Value) -> Result<(), String> {
        let VarSlot::Local { index, .. } = var.var else { return Err("unexpected global".into()) };
        self.vars.insert(index, value);
        Ok(())
    }
    fn ref_value(&mut self, r: RefIndex) -> Result<Value, String> {
        match self.refs.get(r.0 as usize - 1) {
            Some(RefSlot::Form(id)) => Ok(Value::Ref(*id)),
            _ => Err("unexpected reference".into()),
        }
    }
    fn call(&mut self, opcode: u16, on: Option<Value>, args: &[ArgValue]) -> Result<Value, String> {
        assert_eq!(opcode, 0x100E, "GetActorValue is the only fixture host call");
        assert_eq!(on, Some(Value::Ref(PLAYER)));
        let Value::Num(code) = args.first().and_then(ArgValue::value).ok_or("no numeric argument")? else {
            return Err("enum argument must retain its evaluated number".into());
        };
        self.codes.push(code);
        Ok(Value::Num(code + 100.0))
    }
    fn ext(&mut self) -> Option<&mut ExtStore> { Some(&mut self.ext) }
}

fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap()
}

#[test]
fn override_passes_the_changing_enum_value_through_the_real_vm() {
    let table = table();
    let source = "int code\nfloat result\nBegin _GameMode\nlet code := 15\nwhile (code += 1) < 19\nset result to Player.GetAV code\nloop\nEnd";
    let c = compile(source, &table, &Forms, None);
    let errors: Vec<_> = c.errors().collect();
    assert!(errors.is_empty(), "compile errors: {errors:?}");
    let decoded = decode(&c.bytes, &table);
    assert_eq!(decoded.report.raw_total(), 0);
    let mut host = State { refs: c.refs, ..State::default() };
    Vm::new(&table, &mut host, RunOpts { strict: true, ..RunOpts::default() }).run(&decoded.program.blocks[0].body).unwrap();
    assert_eq!(host.codes, [16.0, 17.0, 18.0]);
    let result = c.variables.iter().find(|v| v.name == "result").unwrap().index as u16;
    assert_eq!(host.vars[&result], Value::Num(118.0));
}

#[test]
fn override_keeps_named_enum_literals_and_refuses_unknown_names() {
    let table = table();
    let c = compile("Begin _GameMode\nPlayer.GetAV Health\nEnd", &table, &Forms, None);
    assert_eq!(c.errors().count(), 0);
    let decoded = decode(&c.bytes, &table);
    let mut host = State { refs: c.refs, ..State::default() };
    Vm::new(&table, &mut host, RunOpts { strict: true, ..RunOpts::default() }).run(&decoded.program.blocks[0].body).unwrap();
    assert_eq!(host.codes, [16.0]);
    for argument in ["MissingCode", "X", "\"MissingCode\""] {
        let bad = compile(&format!("Begin _GameMode\nPlayer.GetAV {argument}\nEnd"), &table, &Forms, None);
        assert!(bad.errors().next().is_some(), "override is not permission to invent missing names: {argument}");
    }
}

#[test]
fn vanilla_enum_arguments_do_not_gain_override_semantics() {
    let table = table();
    let c = compile("int code\nBegin GameMode\nPlayer.GetAV code\nEnd", &table, &Forms, None);
    assert!(c.errors().any(|e| e.kind == "unknown_enum_name"));
}
