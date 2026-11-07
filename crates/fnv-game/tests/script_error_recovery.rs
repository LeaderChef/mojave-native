//! Scheduled scripts retain their eligibility and state after a VM error.
mod common;

use common::*;
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_game::{game::{Game, GameConfig}, trace::EventKind, GameData};
use fnv_script::{vm::XVal, FunctionTable};
use fnv_sim::{FormId, FormStore, RefTable, World};

const QUEST: FormId = FormId(0x900);
const SCRIPT: u32 = 0x901;

struct Fixture { data: GameData, forms: FormStore, refs: RefTable }

fn fixture() -> Fixture {
    let source = "scn RecoveryScript\nshort Attempts\nfloat Divisor\nfloat Result\nshort Completed\nBegin GameMode\nset Attempts to Attempts + 1\nwhile Divisor == 0\nloop\nlet Result := 100 / Divisor\nset Completed to Completed + 1\nEnd\n";
    // Unknown bytecode requests the same native source fallback as extender mods.
    let mut script_subs = vec![sub(b"EDID", z("RecoveryScript"))];
    script_subs.extend(unit(1, &script(&[(0, None, st(0x7FFF, &[]))]),
        &[(1, "Attempts", 1), (2, "Divisor", 0), (3, "Result", 0), (4, "Completed", 1)], &[]));
    script_subs.push(sub(b"SCTX", source.as_bytes().to_vec()));
    let quest = record(b"QUST", 0, QUEST.0, &[
        sub(b"EDID", z("RecoveryQuest")), sub(b"SCRI", SCRIPT.to_le_bytes().to_vec()),
        sub(b"DATA", [vec![1, 0, 0, 0], 0.5f32.to_le_bytes().to_vec()].concat()),
    ]);
    let bytes = [tes4(), top(b"QUST", &quest), top(b"SCPT", &record(b"SCPT", 0, SCRIPT, &script_subs))].concat();
    let plugins = vec![Plugin::parse(&bytes, "AuthoredRecovery.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/fnv")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/script/functions.json");
    let catalogue = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let forms = FormStore::build(&lo, &schema);
    let refs = RefTable::build(&lo, &schema);
    let data = GameData::build(&lo, &schema, FunctionTable::load(&path).unwrap(), &catalogue, &refs);
    assert_eq!(data.issues.count("script_compiled_from_source"), 1, "{:?}", data.issues);
    Fixture { data, forms, refs }
}

fn config(strict: bool) -> GameConfig { GameConfig { strict, frame_seconds: 0.5, max_steps: 64, ..Default::default() } }

fn q(g: &mut Game, body: &str) -> XVal {
    g.run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), Default::default()).unwrap()
}

fn errors(g: &Game) -> usize {
    g.events.iter().filter(|e| matches!(e.kind, EventKind::ScriptError { .. })).count()
}

fn number(g: &mut Game, name: &str) -> XVal {
    q(g, &format!("SetFunctionValue RecoveryQuest.{name}"))
}

#[test]
fn quest_recovers_after_error_and_after_save_without_erasing_errors_or_explicit_stop() {
    let f = fixture();
    for strict in [false, true] {
        let mut g = Game::new(&f.data, World::new(&f.forms, &f.refs, Default::default()), config(strict));
        g.new_game();
        assert!(g.quest_state(QUEST).running);
        g.tick();
        assert_eq!(errors(&g), 1, "a budget-limited loop failure must remain visible, strict={strict}");
        assert_eq!(number(&mut g, "Attempts"), XVal::Num(1.0));
        assert_eq!(number(&mut g, "Completed"), XVal::Num(0.0));
        assert!(g.quest_state(QUEST).running, "a VM error must not stop its quest");
        q(&mut g, "set RecoveryQuest.Divisor to 4");
        g.tick();
        assert_eq!(number(&mut g, "Attempts"), XVal::Num(2.0));
        assert_eq!(number(&mut g, "Completed"), XVal::Num(1.0));
        assert_eq!(number(&mut g, "Result"), XVal::Num(25.0));
        assert_eq!(errors(&g), 1, "the earlier error is neither repeated nor erased");

        q(&mut g, "set RecoveryQuest.Divisor to 0");
        g.tick();
        assert_eq!(errors(&g), 2);
        let world = serde_json::from_slice(&serde_json::to_vec(&g.world.snapshot()).unwrap()).unwrap();
        let save = serde_json::from_slice(&serde_json::to_vec(&g.snapshot()).unwrap()).unwrap();
        let mut loaded = Game::restore(&f.data, World::restore(&f.forms, &f.refs, &world).unwrap(), config(strict), &save).unwrap();
        q(&mut loaded, "set RecoveryQuest.Divisor to 5");
        loaded.tick();
        assert_eq!(number(&mut loaded, "Attempts"), XVal::Num(4.0), "failed-run locals survive load");
        assert_eq!(number(&mut loaded, "Completed"), XVal::Num(2.0));
        assert_eq!(number(&mut loaded, "Result"), XVal::Num(20.0));
        assert_eq!(errors(&loaded), 0);

        q(&mut loaded, "StopQuest RecoveryQuest");
        loaded.tick();
        assert!(!loaded.quest_state(QUEST).running);
        assert_eq!(number(&mut loaded, "Attempts"), XVal::Num(4.0), "an intentional stop must still stop execution");
        q(&mut loaded, "StartQuest RecoveryQuest\nToggleScripts");
        loaded.tick();
        assert_eq!(number(&mut loaded, "Attempts"), XVal::Num(4.0), "the global script toggle is not an error lock");
        q(&mut loaded, "ToggleScripts");
        loaded.tick();
        assert_eq!(number(&mut loaded, "Completed"), XVal::Num(3.0));
    }
}

#[test]
fn optional_query_reports_native_error_recovery_and_cannot_enable_permanent_locks() {
    let f = fixture();
    let mut g = Game::new(&f.data, World::new(&f.forms, &f.refs, Default::default()), config(false));
    g.new_game();
    let query = "SetFunctionValue (GetOptionalPatch \"bNoFailedScriptLocks\")";
    assert_eq!(q(&mut g, query), XVal::Num(1.0));
    q(&mut g, "SetOptionalPatch \"bNoFailedScriptLocks\" 1\nSetOptionalPatch \"bNoFailedScriptLocks\" 0");
    assert_eq!(q(&mut g, query), XVal::Num(1.0));
    assert_eq!(q(&mut g, "SetFunctionValue (GetOptionalPatch \"bUnknownRecoveryPatch\")"), XVal::Num(0.0));
    g.tick();
    assert_eq!(errors(&g), 1);
    q(&mut g, "set RecoveryQuest.Divisor to 10");
    g.tick();
    assert_eq!(number(&mut g, "Result"), XVal::Num(10.0), "a disable request cannot reinstate permanent script failure");
    assert_eq!(number(&mut g, "Completed"), XVal::Num(1.0));
}
