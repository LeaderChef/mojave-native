//! Documented animation groups must reach the VM with their actual group codes.
use fnv_script::compile::{compile, Env, FormInfo, PLAYER};
use fnv_script::unit::RefSlot;
use fnv_script::vm::{ArgValue, Host, RunOpts, Value, Vm};
use fnv_script::{decode, FunctionTable, RefIndex, VarDecl, VarRef};

struct Forms;
impl Env for Forms {
    fn form(&self, _: &str) -> Option<FormInfo> { None }
    fn info(&self, _: u32) -> Option<FormInfo> { None }
    fn editor_id(&self, _: u32) -> Option<String> { None }
    fn local_form(&self, local: u32) -> Option<u32> { Some(local) }
    fn script_vars(&self, _: u32) -> Option<Vec<VarDecl>> { None }
}

#[derive(Default)]
struct AnimationHost { refs: Vec<RefSlot>, requests: Vec<(u16, u16)> }
impl Host for AnimationHost {
    fn get_var(&mut self, _: &VarRef) -> Result<Value, String> { Err("fixture has no variables".into()) }
    fn set_var(&mut self, _: &VarRef, _: Value) -> Result<(), String> { Err("fixture has no variables".into()) }
    fn ref_value(&mut self, r: RefIndex) -> Result<Value, String> {
        match self.refs.get(r.0 as usize - 1) {
            Some(RefSlot::Form(id)) => Ok(Value::Ref(*id)),
            _ => Err("unexpected fixture reference".into()),
        }
    }
    fn call(&mut self, opcode: u16, on: Option<Value>, args: &[ArgValue]) -> Result<Value, String> {
        assert_eq!(opcode, 0x1013, "only the observed PlayGroup opcode is used");
        assert_eq!(on, Some(Value::Ref(PLAYER)));
        assert_eq!(args.len(), 2);
        let number = |i: usize| args[i].value().map(|v| v.num() as u16).ok_or_else(|| "missing numeric animation argument".to_string());
        self.requests.push((number(0)?, number(1)?));
        Ok(Value::Num(0.0))
    }
}

fn table() -> FunctionTable {
    FunctionTable::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json")).unwrap()
}

const DOCUMENTED: [(&str, u16); 9] = [
    ("DynamicIdle", 1), ("TurnRight", 16), ("AimIS", 20), ("AttackLeft", 26),
    ("Attack3", 38), ("Attack4", 44), ("BlockHit", 171), ("JumpStart", 227), ("JumpLoop", 228),
];

#[test]
fn documented_animation_names_compile_decode_and_execute_with_each_start_flag() {
    let table = table();
    for (name, code) in DOCUMENTED {
        for flag in 0..=2 {
            let source = format!("Begin GameMode\nPlayer.PlayGroup {name} {flag}\nEnd");
            let c = compile(&source, &table, &Forms, None);
            let errors: Vec<_> = c.errors().collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
            let decoded = decode(&c.bytes, &table);
            assert_eq!(decoded.report.raw_total(), 0);
            let mut host = AnimationHost { refs: c.refs, ..AnimationHost::default() };
            Vm::new(&table, &mut host, RunOpts { strict: true, ..RunOpts::default() }).run(&decoded.program.blocks[0].body).unwrap();
            assert_eq!(host.requests, [(code, flag)]);
        }
    }
}

#[test]
fn all_documented_groups_have_unique_names_and_previous_codes_are_unchanged() {
    let table = table();
    let names: std::collections::BTreeSet<_> = (0..=244).map(|code| {
        table.enum_name("anim_group", code).unwrap_or_else(|| panic!("missing documented animation group {code}")).to_ascii_lowercase()
    }).collect();
    assert_eq!(names.len(), 245);
    for (code, name) in [(0,"Idle"),(2,"SpecialIdle"),(3,"Forward"),(4,"Backward"),(5,"Left"),(6,"Right"),
        (7,"FastForward"),(8,"FastBackward"),(9,"FastLeft"),(10,"FastRight"),(15,"TurnLeft"),(17,"Aim"),(24,"Equip"),(25,"Unequip")] {
        assert_eq!(table.enum_name("anim_group",code),Some(name));
    }
    let bad = compile("Begin GameMode\nPlayer.PlayGroup JumpStartExtra 1\nEnd", &table, &Forms, None);
    assert!(bad.errors().any(|e| e.kind == "unknown_enum_name"));
}
