//! Own source -> compiler -> bytecode -> IR -> VM regressions for Eval/TestExpr.
use fnv_script::compile::{compile, Env, FormInfo};
use fnv_script::unit::RefSlot;
use fnv_script::vm::{ArgValue, ExtStore, Host, RunOpts, Value, Vm};
use fnv_script::{decode, FunctionTable, RefIndex, VarDecl, VarRef, VarSlot};
use std::collections::BTreeMap;

const TARGET: u32 = 0x0100_0800;
struct Forms;
impl Env for Forms {
    fn form(&self, name: &str) -> Option<FormInfo> {
        name.eq_ignore_ascii_case("ProbeTarget").then_some(FormInfo { form: TARGET, sig: *b"REFR" })
    }
    fn info(&self, form: u32) -> Option<FormInfo> {
        (form == TARGET).then_some(FormInfo { form, sig: *b"REFR" })
    }
    fn editor_id(&self, form: u32) -> Option<String> { (form == TARGET).then(|| "ProbeTarget".into()) }
    fn local_form(&self, local: u32) -> Option<u32> { Some(local) }
    fn script_vars(&self, _: u32) -> Option<Vec<VarDecl>> { None }
}

#[derive(Default)]
struct State {
    vars: BTreeMap<u16, Value>,
    refs: Vec<RefSlot>,
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
            Some(RefSlot::Form(form)) => Ok(Value::Ref(*form)),
            _ => Err("unexpected reference".into()),
        }
    }
    fn call(&mut self, _: u16, _: Option<Value>, _: &[ArgValue]) -> Result<Value, String> {
        Err("fixture must use the actual expression VM, not a host fallback".into())
    }
    fn ext(&mut self) -> Option<&mut ExtStore> { Some(&mut self.ext) }
}

fn execute(body: &str) -> BTreeMap<String, Value> {
    let table = FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap();
    let src = format!("scn EvalProbe\nint n\nint other\nint result\nref target\narray_var a\nbegin GameMode\n{body}\nend\n");
    let c = compile(&src, &table, &Forms, None);
    let errors: Vec<_> = c.errors().collect();
    assert!(errors.is_empty(), "compile errors: {errors:?}");
    let decoded = decode(&c.bytes, &table);
    assert_eq!(decoded.report.raw_total(), 0);
    let mut state = State { refs: c.refs.clone(), ..State::default() };
    Vm::new(&table, &mut state, RunOpts { strict: true, ..RunOpts::default() })
        .run(&decoded.program.blocks[0].body).expect("real VM executes the assignments");
    c.variables.iter().map(|v| (v.name.clone(), state.vars.get(&(v.index as u16)).copied().unwrap_or_default())).collect()
}

#[test]
fn eval_owns_numeric_reference_and_nested_assignments() {
    let vars = execute("eval n = 4\neval n := other := n * (3 - 1)\neval target = ProbeTarget\nlet result := Eval n = 0");
    assert_eq!(vars["n"], Value::Num(0.0));
    assert_eq!(vars["other"], Value::Num(8.0));
    assert_eq!(vars["target"], Value::Ref(TARGET));
    assert_eq!(vars["result"], Value::Num(0.0), "Eval returns the assigned value's boolean");
}

#[test]
fn testexpr_retains_assignment_effects_and_its_success_boolean() {
    let vars = execute("eval n = 6\nlet result := TestExpr n = 0\nlet other := TestExpr a[9] := 1");
    assert_eq!(vars["n"], Value::Num(0.0));
    assert_eq!(vars["result"], Value::Num(1.0), "assigning zero succeeds");
    assert_eq!(vars["other"], Value::Num(0.0), "an invalid array assignment remains a caught error");
}
