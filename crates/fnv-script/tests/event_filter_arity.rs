//! Repeated documented catalogue slots survive source -> bytecode -> IR -> VM.
//! Own fixtures only; no mod source or extender implementation is included.

use fnv_script::compile::{compile, Compiled, Env, FormInfo, PLAYER};
use fnv_script::ir::{FuncId, XCall, XExpr, XOp};
use fnv_script::unit::RefSlot;
use fnv_script::vm::{ArgValue, EventCall, ExtStore, Host, RunOpts, Value, Vm, XKey, XVal};
use fnv_script::{decode, FunctionTable, Program, RefIndex, Stmt, VarDecl, VarRef};

const HANDLER: u32 = 0x0100_1000;

struct Forms;

impl Env for Forms {
    fn form(&self, editor_id: &str) -> Option<FormInfo> {
        editor_id.eq_ignore_ascii_case("ProbeHandler").then_some(FormInfo { form: HANDLER, sig: *b"SCPT" })
    }
    fn info(&self, form: u32) -> Option<FormInfo> {
        (form == HANDLER).then_some(FormInfo { form, sig: *b"SCPT" })
    }
    fn editor_id(&self, form: u32) -> Option<String> {
        (form == HANDLER).then(|| "ProbeHandler".into())
    }
    fn local_form(&self, local: u32) -> Option<u32> { Some(local) }
    fn script_vars(&self, _form: u32) -> Option<Vec<VarDecl>> { None }
}

fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json"))
        .expect("documented catalogue loads")
}

fn source(line: &str) -> String {
    format!("scn FilterArityProbe\nbegin GameMode\n{line}\nend\n")
}

fn build(table: &FunctionTable, line: &str) -> (Compiled, Program) {
    let compiled = compile(&source(line), table, &Forms, None);
    let errors: Vec<_> = compiled.errors().collect();
    assert!(errors.is_empty(), "actual compile errors: {errors:?}");
    let decoded = decode(&compiled.bytes, table);
    assert!(decoded.report.issues.is_empty(), "decode issues: {:?}", decoded.report.issues);
    assert_eq!(decoded.report.raw_total(), 0);
    (compiled, decoded.program)
}

fn call(program: &Program) -> &XCall {
    let [Stmt::Eval { expr: XExpr::Call { call } }] = &program.blocks[0].body[..]
        else { panic!("expected one complete event call: {:?}", program.blocks[0].body) };
    call
}

fn pair(expr: &XExpr) -> (&XExpr, &XExpr) {
    let XExpr::Binary { op: XOp::Pair, l, r } = expr else { panic!("whole pair missing: {expr:?}") };
    (l, r)
}

fn numeric(expr: &XExpr) -> f64 {
    let XExpr::Num { value, .. } = expr else { panic!("numeric expression expected: {expr:?}") };
    *value
}

#[derive(Default)]
struct Events {
    refs: Vec<RefSlot>,
    store: ExtStore,
    registrations: Vec<EventCall>,
    live_array_filters: Vec<(u32, Vec<(XKey, XVal)>)>,
}

impl Host for Events {
    fn get_var(&mut self, _var: &VarRef) -> Result<Value, String> { Err("no variable read in fixture".into()) }
    fn set_var(&mut self, _var: &VarRef, _value: Value) -> Result<(), String> { Err("no variable write in fixture".into()) }
    fn ref_value(&mut self, reference: RefIndex) -> Result<Value, String> {
        match self.refs.get(reference.0 as usize - 1) {
            Some(RefSlot::Form(form)) => Ok(Value::Ref(*form)),
            _ => Err("reference not present in compiled unit".into()),
        }
    }
    fn call(&mut self, _opcode: u16, _on: Option<Value>, _args: &[ArgValue]) -> Result<Value, String> {
        Err("fixture must not issue a vanilla call".into())
    }
    fn ext(&mut self) -> Option<&mut ExtStore> { Some(&mut self.store) }
    fn event_call(&mut self, event: EventCall) -> Result<XVal, String> {
        // Inspect temporary arrays while the actual VM still owns them. This fixture
        // records the event-host delivery; it does not retain an array for a game host.
        if let EventCall::Set { pairs, .. } = &event {
            for (_, value) in pairs {
                if let XVal::Array(array) = value {
                    let entries = self.store.arrays.get(array)
                        .ok_or("event-host received an already-disposed array")?.entries.clone();
                    self.live_array_filters.push((*array, entries));
                }
            }
        }
        self.registrations.push(event);
        Ok(XVal::Num(1.0))
    }
}

fn execute(table: &FunctionTable, compiled: &Compiled, program: &Program) -> Events {
    let mut host = Events { refs: compiled.refs.clone(), ..Events::default() };
    Vm::new(table, &mut host, RunOpts { strict: true, ..RunOpts::default() })
        .run(&program.blocks[0].body).expect("actual VM event call succeeds");
    host
}

#[test]
fn alt_three_filters_and_priority_reach_the_vm_as_whole_pairs() {
    let table = table();
    let (compiled, program) = build(&table,
        "SetEventHandlerAlt \"J6:Probe\" ProbeHandler 0::(Ar_List Player, 7) 1::Player 2::1 \"priority\"::2");
    let call = call(&program);
    assert!(matches!(&call.func, FuncId::Name(name) if name.eq_ignore_ascii_case("SetEventHandlerAlt")));
    assert_eq!(call.args.len(), 6, "event, handler, three filters, priority");
    for (index, expected) in [(2, 0.0), (3, 1.0), (4, 2.0)] {
        assert_eq!(numeric(pair(&call.args[index]).0), expected);
    }
    assert!(matches!(pair(&call.args[2]).1, XExpr::Call { call }
        if matches!(&call.func, FuncId::Name(name) if name.eq_ignore_ascii_case("Ar_List")) && call.args.len() == 2));
    assert!(matches!(pair(&call.args[5]).0, XExpr::Str { s } if s == "priority"));
    assert_eq!(numeric(pair(&call.args[5]).1), 2.0);

    let host = execute(&table, &compiled, &program);
    let [EventCall::Set { event, func, pairs, alt }] = &host.registrations[..] else { panic!("one registration expected") };
    assert_eq!(event, "J6:Probe");
    assert_eq!(*func, XVal::Ref(HANDLER));
    assert!(*alt);
    assert_eq!(pairs.len(), 4);
    let (XVal::Num(0.0), XVal::Array(array)) = &pairs[0] else { panic!("array filter was not preserved") };
    assert_eq!(host.live_array_filters, vec![(*array,
        vec![(XKey::Num(0.0), XVal::Ref(PLAYER)), (XKey::Num(1.0), XVal::Num(7.0))])]);
    assert!(!host.store.arrays.contains_key(array), "unretained temporary is disposed after VM exit");
    assert_eq!(pairs[1], (XVal::Num(1.0), XVal::Ref(PLAYER)));
    assert_eq!(pairs[2], (XVal::Num(2.0), XVal::Num(1.0)));
    assert_eq!(pairs[3], (XVal::Str("priority".into()), XVal::Num(2.0)));
}

#[test]
fn normal_two_filters_and_priority_reach_the_vm_without_truncation() {
    let table = table();
    let (compiled, program) = build(&table,
        "SetEventHandler \"J6:Probe\" ProbeHandler \"priority\"::2 \"first\"::Player \"second\"::1");
    let call = call(&program);
    assert_eq!(call.args.len(), 5, "priority may precede both whole filter pairs");
    for (index, key) in [(2, "priority"), (3, "first"), (4, "second")] {
        assert!(matches!(pair(&call.args[index]).0, XExpr::Str { s } if s == key));
    }
    let host = execute(&table, &compiled, &program);
    let [EventCall::Set { pairs, alt, .. }] = &host.registrations[..] else { panic!("one registration expected") };
    assert!(!alt);
    assert_eq!(pairs, &vec![(XVal::Str("priority".into()), XVal::Num(2.0)),
        (XVal::Str("first".into()), XVal::Ref(PLAYER)), (XVal::Str("second".into()), XVal::Num(1.0))]);
}

#[test]
fn malformed_pair_and_unknown_function_remain_compile_errors() {
    let table = table();
    for line in ["SetEventHandlerAlt \"J6:Probe\" ProbeHandler 0::Player 1::1 2::2 \"priority\"::",
        "DefinitelyUnknownJ6Function 1 2 3"] {
        let compiled = compile(&source(line), &table, &Forms, None);
        assert!(compiled.errors().next().is_some(), "invalid input must remain an error: {line}");
    }
}
