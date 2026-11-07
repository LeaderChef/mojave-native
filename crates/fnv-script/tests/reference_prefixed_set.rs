//! Authored caller/assignment ownership regressions; no original script payloads.
use fnv_script::{compile::{compile, Compiled, Env, FormInfo, PLAYER}, decode, ir::*, unit::RefSlot,
    vm::{ArgValue, Host, RunOpts, Value, Vm}, FunctionTable, VarDecl, VarType};
use std::collections::BTreeMap;

const VOICE: u32 = 0x0100_0800;
const QUEST: u32 = 0x0100_0900;

struct Names;
impl Env for Names {
    fn form(&self, name: &str) -> Option<FormInfo> {
        let (form, sig) = match name.to_ascii_lowercase().as_str() {
            "voiceref" => (VOICE, *b"REFR"), "targetquest" => (QUEST, *b"QUST"), _ => return None,
        };
        Some(FormInfo { form, sig })
    }
    fn info(&self, form: u32) -> Option<FormInfo> {
        match form { VOICE => self.form("VoiceRef"), QUEST => self.form("TargetQuest"), _ => None }
    }
    fn editor_id(&self, form: u32) -> Option<String> {
        match form { VOICE => Some("VoiceRef".into()), QUEST => Some("TargetQuest".into()), _ => None }
    }
    fn local_form(&self, form: u32) -> Option<u32> { Some(form) }
    fn script_vars(&self, form: u32) -> Option<Vec<VarDecl>> {
        (form == QUEST).then(|| vec![VarDecl { index: 3, name: "timer".into(), ty: VarType::FloatOrRef }])
    }
}
fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap()
}
fn compiled(source: &str) -> Compiled {
    let result = compile(source, &table(), &Names, None);
    assert_eq!(result.errors().count(), 0, "{:?}", result.diags);
    result
}
fn program(c: &Compiled) -> Program {
    let decoded = decode(&c.bytes, &table());
    assert_eq!(decoded.report.raw_total(), 0, "{:?}", decoded.report.issues);
    decoded.program
}

#[derive(Default)]
struct Calls {
    refs: Vec<RefSlot>, values: BTreeMap<(Option<u16>, u16), Value>,
    callers: Vec<Option<Value>>, fail_next: bool,
}
impl Host for Calls {
    fn get_var(&mut self, var: &VarRef) -> Result<Value, String> {
        let VarSlot::Local { index, .. } = var.var else { return Err("unexpected global".into()) };
        Ok(self.values.get(&(var.on.map(|r| r.0), index)).copied().unwrap_or(Value::ZERO))
    }
    fn set_var(&mut self, var: &VarRef, value: Value) -> Result<(), String> {
        let VarSlot::Local { index, .. } = var.var else { return Err("unexpected global".into()) };
        self.values.insert((var.on.map(|r| r.0), index), value); Ok(())
    }
    fn ref_value(&mut self, r: RefIndex) -> Result<Value, String> {
        match self.refs.get(r.0 as usize - 1).cloned() {
            Some(RefSlot::Form(f)) => Ok(Value::Ref(f)),
            Some(RefSlot::Local(i)) => self.get_var(&VarRef { on: None, var: VarSlot::Local { index: i as u16, letter: VarLetter::Float } }),
            Some(RefSlot::Unmapped(_)) | None => Err("missing reference".into()),
        }
    }
    fn call(&mut self, _: u16, on: Option<Value>, _: &[ArgValue]) -> Result<Value, String> {
        self.callers.push(on);
        if std::mem::take(&mut self.fail_next) { return Err("authored call failure".into()); }
        Ok(Value::Num(match on { Some(Value::Ref(VOICE)) => 11.0, Some(Value::Ref(PLAYER)) => 20.0, _ => 3.0 }))
    }
}

#[test]
fn reference_prefixed_set_keeps_local_target_and_explicit_callers() {
    let c = compiled("float value\nVoiceRef.SeT value To GetPos X + Player.GetPos X\nset value to GetPos X");
    let p = program(&c);
    let mut host = Calls { refs: c.refs, ..Default::default() };
    let t = table(); let mut vm = Vm::new(&t, &mut host, RunOpts::default());
    vm.run(&p.top[..1]).unwrap();
    vm.run(&p.top[1..]).unwrap();
    assert_eq!(host.callers, [Some(Value::Ref(VOICE)), Some(Value::Ref(PLAYER)), None]);
    assert_eq!(host.values.get(&(None, 1)), Some(&Value::Num(3.0)));
    assert_eq!(host.values.len(), 1, "caller scope must not create a remote variable");
}

#[test]
fn reference_prefixed_set_preserves_an_explicit_remote_target() {
    let c = compiled("VoiceRef.Set TargetQuest.timer to GetPos X");
    let p = program(&c);
    let remote = c.refs.iter().position(|r| *r == RefSlot::Form(QUEST)).unwrap() as u16 + 1;
    let mut host = Calls { refs: c.refs, ..Default::default() };
    Vm::new(&table(), &mut host, RunOpts::default()).run(&p.top).unwrap();
    assert_eq!(host.values.get(&(Some(remote), 3)), Some(&Value::Num(11.0)));
    assert_eq!(host.callers, [Some(Value::Ref(VOICE))]);
    assert_eq!(host.values.len(), 1);
}

#[test]
fn reference_prefixed_set_decodes_the_existing_prefix_and_keeps_legacy_json() {
    let mut c = compiled("float value\nset value to 2");
    let plain = program(&c);
    let legacy = serde_json::to_value(&plain.top[0]).unwrap();
    assert!(legacy.get("on").is_none());
    let old: Stmt = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(old).unwrap(), legacy);
    c.refs.push(RefSlot::Form(VOICE));
    c.bytes.splice(..0, [0x1c, 0, 1, 0]);
    let p = program(&c);
    let json = serde_json::to_value(&p.top[0]).unwrap();
    assert_eq!(json["on"], serde_json::json!(1));
    let Stmt::Set { target, .. } = &p.top[0] else { panic!("Set expected") };
    assert!(target.on.is_none());
    // Prefixes on unrelated control statements remain rejected rather than discarded.
    let raw = decode(&[0x1c, 0, 1, 0, 0x1e, 0, 0, 0], &table());
    assert_eq!(raw.report.raw_total(), 1);
}

#[test]
fn reference_prefixed_set_restores_caller_scope_after_an_expression_error() {
    let c = compiled("float value\nVoiceRef.Set value to GetPos X\nset value to GetPos X");
    let p = program(&c); let t = table();
    let mut host = Calls { refs: c.refs, fail_next: true, ..Default::default() };
    let mut vm = Vm::new(&t, &mut host, RunOpts { strict: true, ..Default::default() });
    assert!(vm.run(&p.top[..1]).is_err());
    vm.run(&p.top[1..]).unwrap();
    assert_eq!(host.callers, [Some(Value::Ref(VOICE)), None]);
    assert_eq!(host.values.get(&(None, 1)), Some(&Value::Num(3.0)));
}

#[test]
fn reference_prefixed_set_rejects_unresolved_callers_and_extended_rewrites() {
    for source in ["float value\nMissingRef.Set value to 2", "float value\nVoiceRef.Set value to GetBaseObject"] {
        let c = compile(source, &table(), &Names, None);
        assert!(c.errors().count() > 0, "unsupported spelling must not silently discard its caller");
    }
}
