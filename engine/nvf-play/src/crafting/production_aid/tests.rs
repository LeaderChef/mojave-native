//! UI bridges call the actual Game/World order APIs over synthetic records. No
//! renderer, audio device, game install, native window or copied production algorithm.

use super::*;
use bevy::ecs::system::{IntoSystem, System, SystemState};
use fnv_esm::raw::{write_subrecords, Subrecord};
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_sim::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Container, Item, ItemKind};
use fnv_sim::refs::PlacedRef;
use fnv_sim::{FormStore, RefTable, SimConfig};
use std::time::Duration;

const INPUT: FormId = FormId(0x110);
const OUTPUT: FormId = FormId(0x111);
const BENCH_BASE: FormId = FormId(0x210);
const STORAGE_BASE: FormId = FormId(0x211);
const CATEGORY: FormId = FormId(0x212);
const RECIPE: FormId = FormId(0x213);
const BENCH: FormId = FormId(0x310);
const CELL: FormId = FormId(0x410);

struct Fixture {
    runtime: crate::script::Runtime,
    sim: SimState,
    recipe: RecipeDef,
    corner: crate::corner_messages::CornerMessages,
}

fn item() -> Item {
    Item {
        kind: ItemKind::Misc, value: Some(1), money: None, weight: Some(1.0), slots: None,
        projectile: None, health: None, non_playable: false, cant_drop: false,
        armor_class: ArmorClass::Light, power_armor: false, armor_dr: None, armor_dt: None,
        weapon_damage: None, weight_mods: [0.0; 3], drawable: false, not_for_npcs: false,
        thrown: false, quest_item: false, icon: None, placed_count: None, teaches: None,
        cant_be_taken: false, modulates_voice: false, weapon_traits: None,
    }
}

fn fixture() -> Fixture {
    let mut forms = FormStore::default();
    for (id, sig, name, data) in [
        (INPUT, *b"MISC", "OrderInput", BaseData::Item(item())),
        (OUTPUT, *b"MISC", "OrderOutput", BaseData::Item(item())),
        (BENCH_BASE, *b"ACTI", "WorkBench", BaseData::Activator),
        (STORAGE_BASE, *b"CONT", "MetalBox01Empty", BaseData::Container(Container { respawns: false, entries: vec![] })),
        (FormId::PLAYER_BASE, *b"NPC_", "Player", BaseData::Actor(ActorBase { entries: vec![], attributes: Some([5; 7]) })),
    ] {
        forms.sigs.insert(id, sig);
        forms.edids.insert(name.to_ascii_lowercase(), id);
        forms.bases.insert(id, BaseForm { id, sig, edid: Some(name.into()), name: Some(name.into()), script: None, prompt: None, voice_filter: false, data });
    }
    forms.sigs.insert(CATEGORY, *b"RCCT");
    // RCCT is indexed by GameData, not the actual simulation FormStore.
    for (i, (edid, value)) in [("GameYear", 2281.0), ("GameMonth", 9.0), ("GameDay", 18.0), ("GameHour", 12.0), ("GameDaysPassed", 5.0), ("TimeScale", 30.0)].into_iter().enumerate() {
        let id = FormId(0x500 + i as u32);
        forms.sigs.insert(id, *b"GLOB");
        forms.edids.insert(edid.to_ascii_lowercase(), id);
        forms.globals.insert(id, value);
    }
    let mut refs = RefTable::default();
    refs.refs.insert(BENCH, PlacedRef {
        id: BENCH, sig: *b"REFR", edid: None, base: BENCH_BASE, cell: Some(CELL),
        stored_in: None, plugin: 0, flags: 0, in_persistent_group: false,
        pos: [100.0, 0.0, 0.0], rot: [0.0; 3], scale: None, count: None,
        ammo: None, health: None, owner: None, lock: None, teleport: None,
        enable_parent: None, linked: None, patrol_idle: None, patrol_topic: None,
        merchant_container: None, marker_reputation: None, open_by_default: false,
        primitive: None, activate_parents: vec![], parent_activate_only: false,
        prompt: None, radiation: None, radius: None, encounter_zone: None, level_modifier: None,
    });
    let forms: &'static FormStore = Box::leak(Box::new(forms));
    let refs: &'static RefTable = Box::leak(Box::new(refs));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let payload = write_subrecords(&[Subrecord { sig: *b"HEDR", data: [&1.34f32.to_le_bytes()[..], &0i32.to_le_bytes()[..], &0x800u32.to_le_bytes()[..]].concat(), extended: false }]);
    let bytes = [b"TES4".as_slice(), &(payload.len() as u32).to_le_bytes(), &1u32.to_le_bytes(), &0u32.to_le_bytes(), &[0; 4], &15u16.to_le_bytes(), &[0; 2], &payload].concat();
    let plugins = [Plugin::parse(&bytes, "ProductionUi.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let schema = Schema::load_dir(root.join("fnv")).unwrap();
    let functions = root.join("script/functions.json");
    let json = serde_json::from_str(&std::fs::read_to_string(&functions).unwrap()).unwrap();
    let mut data = fnv_game::GameData::build(&lo, &schema, fnv_script::FunctionTable::load(&functions).unwrap(), &json, refs);
    let recipe = RecipeDef { form: RECIPE, name: "Synthetic queued recipe".into(), category: Some(CATEGORY), ingredients: vec![(INPUT, 1)], outputs: vec![(OUTPUT, 2)], ..Default::default() };
    data.edids.insert("workbenchrecipes".into(), CATEGORY);
    data.edid_of.insert(CATEGORY, "WorkbenchRecipes".into());
    data.recipes.insert(RECIPE, recipe.clone());
    data.sigs.insert(RECIPE, *b"RCPE");
    data.sigs.insert(CATEGORY, *b"RCCT");
    data.recipe_categories.insert(CATEGORY, fnv_game::crafting::RecipeCategoryDef { name: "Workbench".into(), subcategory: false });
    let data: &'static fnv_game::GameData = Box::leak(Box::new(data));
    let mut sim = SimState { forms, refs, world: fnv_sim::World::new(forms, refs, SimConfig::default()) };
    let player = sim.world.get(FormId::PLAYER_REF).unwrap();
    player.cell = Some(CELL);
    player.pos = [0.0; 3];
    sim.world.add_item(FormId::PLAYER_REF, INPUT, 4).unwrap();
    let mut state = ScriptState::over_no_plugins();
    let mut runtime = state.rt.take().unwrap();
    runtime.game = fnv_game::Game::new(data, fnv_sim::World::new(forms, refs, SimConfig::default()), fnv_game::GameConfig::default());
    Fixture { runtime, sim, recipe, corner: Default::default() }
}

impl Fixture {
    fn count(&mut self, item: FormId) -> i32 { self.sim.world.item_count(FormId::PLAYER_REF, item).unwrap() }

    fn with_sounds<R>(&mut self, call: impl FnOnce(&mut Self, &mut MessageWriter<UiSound>) -> R) -> (R, Vec<UiSound>) {
        let mut ecs = World::new();
        ecs.init_resource::<Messages<UiSound>>();
        let mut writer = SystemState::<MessageWriter<UiSound>>::new(&mut ecs);
        let result = { let mut sounds = writer.get_mut(&mut ecs).unwrap(); call(self, &mut sounds) };
        writer.apply(&mut ecs);
        (result, ecs.resource_mut::<Messages<UiSound>>().drain().collect())
    }

    fn queue(&mut self, station: Option<FormId>, times: u32) -> (Option<u64>, Vec<UiSound>) {
        self.with_sounds(|f, sounds| queue_recipe(&mut f.runtime, &mut f.sim, station, CATEGORY, &f.recipe, times, sounds, Some(&mut f.corner)))
    }

    fn finish(&mut self, id: u64, collect: bool) -> (bool, Vec<UiSound>) {
        self.with_sounds(|f, sounds| finish_order(&mut f.runtime, &mut f.sim, id, collect, sounds, Some(&mut f.corner)))
    }
}

#[test]
fn recipe_menu_follows_script_renames_and_cloned_subcategory_without_changing_stock() {
    let mut f = fixture();
    let before_stock = f.sim.world.snapshot();
    let mut open = Open { category: CATEGORY, station: Some(BENCH), filter: 0,
        selected: 0, first: 0, drawn: None, quantity: None, order_mode: false };
    let before = view(&mut f.runtime, &mut f.sim, &open);
    assert_eq!(before.rows[0].label, "Synthetic queued recipe (2)");
    assert_eq!(before.filters, 0);

    // Actual native VM mutations, including a session clone absent from GameData.
    let source = "Begin Function { }\nref r := GetSelf\nref c := TempCloneForm WorkbenchRecipes\nLNSetName \"Revised batch\" r\nLNSetName \"Salvaged supplies\" c\nSetRecipeSubcategory r c\nSetFunctionValue c\nEnd";
    let clone = f.runtime.game.run_query(source, fnv_game::game::RunCtx {
        self_ref: Some(RECIPE), ..Default::default()
    }).unwrap();
    let fnv_script::vm::XVal::Ref(clone) = clone else { panic!("expected a category clone") };
    assert_ne!(clone, 0);
    assert_eq!(f.runtime.game.unclone(FormId(clone)), CATEGORY);
    f.runtime.game.run_query("Begin Function { }\nLNSetName \"Sorted ingredient\"\nEnd",
        fnv_game::game::RunCtx { self_ref: Some(INPUT), ..Default::default() }).unwrap();
    assert!(!f.runtime.game.data.recipe_categories.contains_key(&FormId(clone)));

    let all = view(&mut f.runtime, &mut f.sim, &open);
    assert_eq!(all.rows[0].label, "Revised batch (2)");
    assert_eq!(all.filters, 1);
    assert_eq!(all.ingredients, [("Sorted ingredient".into(), 4, 1)]);
    assert!(all.rows[0].ready);
    open.filter = 1;
    let filtered = view(&mut f.runtime, &mut f.sim, &open);
    assert_eq!(filtered.title, "SALVAGED SUPPLIES");
    assert_eq!(filtered.rows, all.rows);

    // Opening that exact runtime subcategory directly uses the same live membership.
    open.category = FormId(clone);
    open.filter = 0;
    assert_eq!(view(&mut f.runtime, &mut f.sim, &open).rows, all.rows);

    // The actual script-call admission must retain the cloned category, not reject
    // it just because that session form is absent from the parsed record map.
    let ctx = fnv_game::functions::Ctx { unit: "recipe view regression".into(), opcode: 0x1250,
        on: Some(BENCH), self_ref: None, action_ref: None, quest: None, inst: None, in_condition: false };
    let argument = [fnv_script::vm::ArgValue::Value(fnv_script::vm::Value::Ref(clone))];
    fnv_game::functions::misc::show_recipe_menu(&mut f.runtime.game, &ctx,
        &fnv_game::functions::Args(&argument)).expect("a real category clone can open its recipe menu");
    assert_eq!(f.runtime.game.recipe_requests.last().unwrap().category, FormId(clone));
    assert_eq!(f.runtime.game.recipe_requests.last().unwrap().station, Some(BENCH));
    let requests = f.runtime.game.recipe_requests.len();
    for invalid in [0, INPUT.0, 0xFF12_3456] {
        let argument = [fnv_script::vm::ArgValue::Value(fnv_script::vm::Value::Ref(invalid))];
        assert!(fnv_game::functions::misc::show_recipe_menu(&mut f.runtime.game, &ctx,
            &fnv_game::functions::Args(&argument)).is_err());
        assert_eq!(f.runtime.game.recipe_requests.len(), requests);
    }

    // A primary-category move also changes membership and the "made at" label.
    let source = "Begin Function { }\nref r := GetSelf\nref c := TempCloneForm WorkbenchRecipes\nLNSetName \"Portable bench\" c\nSetRecipeCategory r c\nSetFunctionValue c\nEnd";
    let moved = f.runtime.game.run_query(source, fnv_game::game::RunCtx {
        self_ref: Some(RECIPE), ..Default::default()
    }).unwrap();
    let fnv_script::vm::XVal::Ref(moved) = moved else { panic!("expected a primary category clone") };
    assert_ne!(moved, 0);
    assert_eq!(f.runtime.game.unclone(FormId(moved)), CATEGORY);
    open.category = CATEGORY;
    assert!(view(&mut f.runtime, &mut f.sim, &open).rows.is_empty());
    open.category = FormId(moved);
    let moved_view = view(&mut f.runtime, &mut f.sim, &open);
    assert_eq!(moved_view.rows, all.rows);
    assert_eq!(moved_view.made_at, "Portable bench");
    assert_eq!(f.sim.world.snapshot(), before_stock);
    assert_eq!(f.runtime.game.data.recipes[&RECIPE], f.recipe);
    assert!(f.runtime.game.production.is_pristine());
}

#[test]
fn player_or_absent_station_never_queues_or_announces_a_success() {
    let mut f = fixture();
    let before = f.sim.world.snapshot(); let game = f.runtime.game.snapshot();
    for station in [None, Some(FormId::PLAYER_REF)] {
        let (id, sounds) = f.queue(station, 1);
        assert_eq!(id, None);
        assert_eq!(sounds, [UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused }]);
        assert_eq!(f.sim.world.snapshot(), before);
        assert_eq!(f.runtime.game.snapshot(), game);
        assert_eq!(f.corner.waiting(), 0);
    }
}

#[test]
fn queue_bridge_reserves_real_sim_stock_without_producing_or_counting_a_craft() {
    let mut f = fixture();
    let (id, sounds) = f.queue(Some(BENCH), 2);
    let id = id.expect("the actual Game admission must accept this valid station");
    assert_eq!(sounds, [UiSound { menu: UiMenu::Recipe, event: UiEvent::Picked }]);
    assert_eq!(f.count(INPUT), 2);
    assert_eq!(f.count(OUTPUT), 0);
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 0.0);
    let order = &f.runtime.game.production.orders[&id];
    assert_eq!(f.sim.world.inventory_reservation(order.escrow), Some(id));
    assert_eq!(f.sim.world.peek(order.escrow).unwrap().inventory.as_ref().unwrap().count(INPUT), 2);
    assert!(f.runtime.game.world.peek(FormId::PLAYER_REF).is_none(), "with_world must leave the actual inventory in SimState");
}

#[test]
fn premature_collect_bridge_refuses_without_any_inventory_or_success_notice() {
    let mut f = fixture(); let id = f.queue(Some(BENCH), 1).0.unwrap();
    let before = f.sim.world.snapshot(); let game = f.runtime.game.snapshot(); let notes = f.corner.waiting();
    let (collected, sounds) = f.finish(id, true);
    assert!(!collected);
    assert_eq!(sounds, [UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused }]);
    assert_eq!(f.sim.world.snapshot(), before);
    assert_eq!(f.runtime.game.snapshot(), game);
    assert_eq!(f.corner.waiting(), notes);
}

#[test]
fn cancel_bridge_returns_real_reserved_inputs_without_output_or_crafting_credit() {
    let mut f = fixture(); let id = f.queue(Some(BENCH), 2).0.unwrap();
    let (cancelled, sounds) = f.finish(id, false);
    assert!(cancelled);
    assert_eq!(sounds, [UiSound { menu: UiMenu::Recipe, event: UiEvent::Back }]);
    assert_eq!((f.count(INPUT), f.count(OUTPUT)), (4, 0));
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 0.0);
}

#[test]
fn completed_collection_bridge_delivers_actual_products_and_counts_completion_only_once() {
    let mut f = fixture(); let id = f.queue(Some(BENCH), 2).0.unwrap();
    let notices = crate::script::with_world(&mut f.runtime, &mut f.sim, |rt| {
        rt.game.world.advance_hours(0.5).unwrap();
        rt.game.advance_production_orders(None)
    });
    assert_eq!(notices.len(), 1);
    assert_eq!(f.count(OUTPUT), 0, "ready output is still in escrow before collection");
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 2.0);
    assert!(f.finish(id, true).0);
    assert_eq!((f.count(INPUT), f.count(OUTPUT)), (2, 4));
    assert!(f.finish(id, true).0, "an idempotent closed-order acknowledgement carries no second delivery");
    assert_eq!((f.count(INPUT), f.count(OUTPUT)), (2, 4));
    assert_eq!(f.runtime.game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 2.0);
}

#[test]
fn actual_view_reads_station_order_and_clock_from_sim_world() {
    let mut f = fixture(); let id = f.queue(Some(BENCH), 1).0.unwrap();
    let open = Open { category: CATEGORY, station: Some(BENCH), filter: 0, selected: 0, first: 0, drawn: None, quantity: None, order_mode: false };
    let v = view(&mut f.runtime, &mut f.sim, &open);
    let order = v.production.unwrap();
    assert_eq!(order.id, id);
    assert!(order.cancel && !order.collect);
    assert!(order.text.contains("15 game minutes"));
    crate::script::with_world(&mut f.runtime, &mut f.sim, |rt| {
        rt.game.world.advance_hours(0.25).unwrap(); rt.game.advance_production_orders(None);
    });
    let v = view(&mut f.runtime, &mut f.sim, &open);
    let order = v.production.unwrap();
    assert!(order.collect && !order.cancel);
    assert!(order.text.contains("ready to collect"));
}

#[test]
fn host_service_uses_real_local_utc_after_loading_and_completes_an_expired_48h_order_once() {
    let mut f = fixture();
    let utc = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let old = utc.checked_sub(fnv_game::production::TWO_DAYS_SECONDS + 5).unwrap();
    let id = crate::script::with_world(&mut f.runtime, &mut f.sim, |rt| {
        rt.game.queue_production_order(BENCH, CATEGORY, RECIPE, 1, 1,
            fnv_game::production::WorkTiming::HostUtc { now: old, seconds: fnv_game::production::TWO_DAYS_SECONDS }).unwrap()
    });
    assert!(f.runtime.game.production.has_host_orders());
    let mut script = ScriptState::over_no_plugins(); script.rt = Some(f.runtime);
    let mut ecs = World::new();
    let mut time = Time::<Real>::default(); time.update_with_duration(Duration::from_secs(1));
    ecs.insert_resource(time);
    ecs.insert_resource(State::new(PlayState::Loading));
    ecs.insert_resource(script); ecs.insert_resource(f.sim);
    ecs.init_resource::<crate::corner_messages::CornerMessages>();
    let mut service = IntoSystem::into_system(host_orders);
    service.initialize(&mut ecs);
    service.run((), &mut ecs).unwrap();
    assert_eq!(ecs.resource::<ScriptState>().rt.as_ref().unwrap().game.production.orders[&id].state, fnv_game::production::OrderState::Working);
    ecs.insert_resource(State::new(PlayState::Playing));
    service.run((), &mut ecs).unwrap();
    let script = ecs.resource::<ScriptState>(); let game = &script.rt.as_ref().unwrap().game;
    assert_eq!(game.production.orders[&id].state, fnv_game::production::OrderState::Ready);
    assert!(game.production.last_host_utc.is_some_and(|now| now >= utc));
    assert_eq!(game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 1.0);
    let escrow = game.production.orders[&id].escrow;
    assert_eq!(ecs.resource::<SimState>().world.peek(escrow).unwrap().inventory.as_ref().unwrap().count(OUTPUT), 2);
    assert_eq!(ecs.resource::<crate::corner_messages::CornerMessages>().waiting(), 1);
    service.run((), &mut ecs).unwrap();
    assert_eq!(ecs.resource::<ScriptState>().rt.as_ref().unwrap().game.misc_stat(fnv_game::misc_stats::ITEMS_CRAFTED), 1.0);
    assert_eq!(ecs.resource::<crate::corner_messages::CornerMessages>().waiting(), 1);
}
