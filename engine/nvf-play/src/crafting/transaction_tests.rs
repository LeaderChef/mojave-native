//! Crafting transaction regressions through the actual menu `make_recipe` method.
//!
//! [design] A refused batch must leave inventories, crafting statistics, challenge
//! state and success notifications unchanged. Duplicate rows describe combined demand
//! or output, not independent chances to consume the same units. Invalid outputs and
//! unrepresentable counts must refuse the whole batch without panicking or truncating.
//! These are transaction invariants, not claims of original-game parity. The plugin is
//! synthetic, decoded by the real schemas; no game, renderer or window is started.

use super::*;
use bevy::ecs::system::SystemState;
use fnv_esm::raw::{write_subrecords, Subrecord};
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_sim::{FormStore, RefTable, SimConfig};
use std::panic::{catch_unwind, AssertUnwindSafe};

const INGREDIENT: FormId = FormId(0x100);
const OTHER_INGREDIENT: FormId = FormId(0x101);
const OUTPUT: FormId = FormId(0x102);
const NOT_AN_ITEM: FormId = FormId(0x103);
const UNKNOWN_OUTPUT: FormId = FormId(0x104);
const INITIAL_CRAFTED: f64 = 4.0;

fn sub(sig: &[u8; 4], data: Vec<u8>) -> Subrecord {
    Subrecord { sig: *sig, data, extended: false }
}

fn z(s: &str) -> Vec<u8> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

fn record(sig: &[u8; 4], flags: u32, fid: FormId, subs: &[Subrecord]) -> Vec<u8> {
    let payload = write_subrecords(subs);
    [
        sig.as_slice(),
        &(payload.len() as u32).to_le_bytes(),
        &flags.to_le_bytes(),
        &fid.0.to_le_bytes(),
        &[0; 4],
        &15u16.to_le_bytes(),
        &[0; 2],
        &payload,
    ].concat()
}

fn top(sig: &[u8; 4], body: &[u8]) -> Vec<u8> {
    [b"GRUP".as_slice(), &((24 + body.len()) as u32).to_le_bytes(), sig, &0i32.to_le_bytes(), &[0; 8], body].concat()
}

fn item(id: FormId, name: &str) -> Vec<u8> {
    record(b"MISC", 0, id, &[
        sub(b"EDID", z(name)),
        sub(b"FULL", z(name)),
        sub(b"DATA", [&1i32.to_le_bytes()[..], &1f32.to_le_bytes()[..]].concat()),
    ])
}

fn plugin_bytes() -> Vec<u8> {
    let header = record(b"TES4", 1, FormId(0), &[
        sub(b"HEDR", [&1.34f32.to_le_bytes()[..], &0i32.to_le_bytes()[..], &0x800u32.to_le_bytes()[..]].concat()),
    ]);
    let items = [item(INGREDIENT, "SyntheticScrap"), item(OTHER_INGREDIENT, "SyntheticBinder"), item(OUTPUT, "SyntheticOutput")].concat();
    let player = record(b"NPC_", 0, FormId::PLAYER_BASE, &[
        sub(b"EDID", z("Player")),
        sub(b"DATA", [&100i32.to_le_bytes()[..], &[5u8; 7][..]].concat()),
    ]);
    let nonitem = record(b"STAT", 0, NOT_AN_ITEM, &[sub(b"EDID", z("SyntheticStatic"))]);
    [header, top(b"MISC", &items), top(b"NPC_", &player), top(b"STAT", &nonitem)].concat()
}

struct Fixture {
    runtime: crate::script::Runtime,
    sim: SimState,
    corner: crate::corner_messages::CornerMessages,
}

fn fixture(items: &[(FormId, i32)]) -> Fixture {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let plugins = [Plugin::parse(&plugin_bytes(), "CraftingTransactions.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(root.join("fnv")).unwrap();
    let forms: &'static FormStore = Box::leak(Box::new(FormStore::build(&lo, &schema)));
    let refs: &'static RefTable = Box::leak(Box::new(RefTable::build(&lo, &schema)));
    assert!(forms.issues.is_empty(), "{:?}", forms.issues.to_json());
    let functions = root.join("script/functions.json");
    let json = serde_json::from_str(&std::fs::read_to_string(&functions).unwrap()).unwrap();
    let data: &'static fnv_game::GameData = Box::leak(Box::new(fnv_game::GameData::build(&lo, &schema, fnv_script::FunctionTable::load(&functions).unwrap(), &json, refs)));
    let mut sim = SimState { forms, refs, world: fnv_sim::World::new(forms, refs, SimConfig::default()) };
    // Materialize even an empty player before the baseline snapshot: lazy inventory
    // initialization is unrelated to a recipe's transaction effects.
    sim.world.inventory_mut(FormId::PLAYER_REF).unwrap();
    for &(item, count) in items {
        sim.world.add_item(FormId::PLAYER_REF, item, count).unwrap();
    }
    // Reuse the existing test-only constructor for private runtime bookkeeping, then
    // give its public game the very same synthetic tables as the actual SimState.
    let mut state = crate::script::ScriptState::over_no_plugins();
    let mut runtime = state.rt.take().unwrap();
    runtime.game = fnv_game::Game::new(data, fnv_sim::World::new(forms, refs, SimConfig::default()), fnv_game::GameConfig::default());
    runtime.game.count_stat(fnv_game::misc_stats::ITEMS_CRAFTED, INITIAL_CRAFTED);
    Fixture { runtime, sim, corner: Default::default() }
}

fn recipe(ingredients: &[(FormId, u32)], outputs: &[(FormId, u32)]) -> RecipeDef {
    RecipeDef {
        form: FormId(0x200),
        name: "Synthetic transaction recipe".into(),
        ingredients: ingredients.to_vec(),
        outputs: outputs.to_vec(),
        ..Default::default()
    }
}

impl Fixture {
    fn count(&mut self, item: FormId) -> i32 {
        self.sim.world.item_count(FormId::PLAYER_REF, item).unwrap()
    }

    fn make(&mut self, recipe: &RecipeDef, times: u32) -> Vec<UiSound> {
        // Only the message writer needs ECS machinery; no App/plugins/schedules or
        // native game are created. The called production method owns every mutation.
        let mut ecs = World::new();
        ecs.init_resource::<Messages<UiSound>>();
        let mut writer = SystemState::<MessageWriter<UiSound>>::new(&mut ecs);
        let result = {
            let mut sounds = writer.get_mut(&mut ecs).unwrap();
            catch_unwind(AssertUnwindSafe(|| {
                make_recipe(&mut self.runtime, &mut self.sim, recipe, times, &mut sounds, Some(&mut self.corner));
            }))
        };
        assert!(result.is_ok(), "a malformed recipe must refuse without unwinding the production world swap");
        writer.apply(&mut ecs);
        ecs.resource_mut::<Messages<UiSound>>().drain().collect()
    }

    fn assert_refused_unchanged(&mut self, recipe: &RecipeDef, times: u32) {
        let inventory_before = self.sim.world.snapshot();
        let game_before = self.runtime.game.snapshot();
        let sounds = self.make(recipe, times);
        assert_eq!(self.sim.world.snapshot(), inventory_before, "refused batch partially consumed ingredients or inserted outputs");
        assert_eq!(self.runtime.game.snapshot(), game_before, "refused batch changed crafting statistics, challenge state or runtime world");
        assert_eq!(sounds, vec![UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused }]);
        assert_eq!(self.corner.waiting(), 0, "refused batch must not announce successful output");
        assert!(self.corner.showing().is_none());
    }
}

#[test]
fn duplicate_ingredient_rows_do_not_mint_output_from_short_total() {
    let mut f = fixture(&[(INGREDIENT, 3)]);
    // Each row alone fits 3; their combined demand of 4 does not. The original path
    // tests each against 3 then ignores how many the second remove actually took.
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 2), (INGREDIENT, 2)], &[(OUTPUT, 1)]), 1);
}

#[test]
fn unknown_second_output_does_not_commit_first_output_or_ingredients() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 2)], &[(OUTPUT, 1), (UNKNOWN_OUTPUT, 1)]), 1);
}

#[test]
fn nonitem_second_output_does_not_commit_first_output_or_ingredients() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    assert_eq!(f.sim.forms.sig(NOT_AN_ITEM), Some(*b"STAT"));
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 2)], &[(OUTPUT, 1), (NOT_AN_ITEM, 1)]), 1);
}

#[test]
fn output_above_signed_count_range_refuses_without_consuming() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 1)], &[(OUTPUT, i32::MAX as u32 + 1)]), 1);
}

#[test]
fn output_product_overflow_refuses_without_panicking_or_consuming() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 1)], &[(OUTPUT, u32::MAX)]), 2);
}

#[test]
fn duplicate_output_rows_must_fit_combined_destination_capacity() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    // Both rows fit i32 separately, but their combined result does not. Saturating
    // storage is not successful delivery of the recipe's advertised whole output.
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 1)], &[(OUTPUT, 1), (OUTPUT, i32::MAX as u32)]), 1);
}

#[test]
fn output_exceeding_existing_destination_capacity_does_not_discard_inputs() {
    let mut f = fixture(&[(INGREDIENT, 5), (OUTPUT, i32::MAX)]);
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 1)], &[(OUTPUT, 1)]), 1);
}

#[test]
fn ordinary_shortage_refuses_without_side_effects() {
    let mut f = fixture(&[(INGREDIENT, 1)]);
    f.assert_refused_unchanged(&recipe(&[(INGREDIENT, 2)], &[(OUTPUT, 1)]), 1);
}

#[test]
fn normal_multibatch_consumes_and_delivers_exact_counts_once() {
    let mut f = fixture(&[(INGREDIENT, 5), (OTHER_INGREDIENT, 4)]);
    let sounds = f.make(&recipe(&[(INGREDIENT, 2), (OTHER_INGREDIENT, 1)], &[(OUTPUT, 3)]), 2);
    assert_eq!(f.count(INGREDIENT), 1);
    assert_eq!(f.count(OTHER_INGREDIENT), 2);
    assert_eq!(f.count(OUTPUT), 6);
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), INITIAL_CRAFTED + 2.0);
    assert_eq!(sounds, vec![UiSound { menu: UiMenu::Recipe, event: UiEvent::Picked }]);
    assert_eq!(f.corner.waiting(), 1, "one success line for the one output type");
}

#[test]
fn sufficient_duplicate_ingredient_rows_charge_the_combined_total() {
    let mut f = fixture(&[(INGREDIENT, 7)]);
    let sounds = f.make(&recipe(&[(INGREDIENT, 2), (INGREDIENT, 2)], &[(OUTPUT, 1)]), 1);
    assert_eq!(f.count(INGREDIENT), 3);
    assert_eq!(f.count(OUTPUT), 1);
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), INITIAL_CRAFTED + 1.0);
    assert_eq!(sounds, vec![UiSound { menu: UiMenu::Recipe, event: UiEvent::Picked }]);
    assert_eq!(f.corner.waiting(), 1);
}

#[test]
fn stale_recipe_condition_refuses_before_consuming_or_delivering() {
    use fnv_game::cond::{Cmp, Condition, Operand, RunOn};
    use fnv_script::vm::{ArgValue, Value};

    let mut f = fixture(&[(INGREDIENT, 5), (OTHER_INGREDIENT, 2)]);
    let mut r = recipe(&[(INGREDIENT, 2)], &[(OUTPUT, 1)]);
    // Use the real condition evaluator and its function catalogue, not a synthetic
    // ready flag. Binder is a visibility requirement, independent of recipe inputs.
    r.conditions.push(Condition {
        cmp: Cmp::Ge,
        or: false,
        run_on: RunOn::Subject,
        function: f.runtime.game.data.table.by_name("GetItemCount").unwrap() - 0x1000,
        params: vec![ArgValue::Value(Value::Ref(OTHER_INGREDIENT.0))],
        rhs: Operand::Value(2.0),
    });
    assert!(crate::script::with_world(&mut f.runtime, &mut f.sim, |rt| {
        rt.game.conditions_pass(&r.conditions, Some(FormId::PLAYER_REF), None, "recipe test selection")
    }), "the selected recipe was actually available before the external change");
    assert_eq!(f.sim.world.remove_item(FormId::PLAYER_REF, OTHER_INGREDIENT, 1).unwrap(), 1);
    assert!(!crate::script::with_world(&mut f.runtime, &mut f.sim, |rt| {
        rt.game.conditions_pass(&r.conditions, Some(FormId::PLAYER_REF), None, "recipe test stale")
    }), "the same recipe now fails its real condition while ingredients remain sufficient");
    // Snapshot after that external change and condition observation. Only the commit
    // attempt itself must be effect-free; do not mistake the removed Binder for loss.
    f.assert_refused_unchanged(&r, 1);
}

#[test]
fn zero_times_empty_batch_refuses_without_success_or_state_change() {
    let mut f = fixture(&[(INGREDIENT, 5)]);
    // A zero request must be rejected explicitly even when no remove/add call would
    // incidentally reject a count of zero. Never report success for an empty batch.
    f.assert_refused_unchanged(&recipe(&[], &[]), 0);
}
