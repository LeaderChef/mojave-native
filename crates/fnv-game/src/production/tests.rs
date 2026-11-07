//! [design] Actual Game/World production transactions and cold saves. The fixture
//! uses declared synthetic forms and the real game-data builder; no second ledger.

#[path = "../../tests/common/mod.rs"]
mod common;

use super::{OrderState, WorkDeadline, WorkTiming, GAME_HOURS_PER_BATCH, TWO_DAYS_SECONDS};
use crate::crafting::{RecipeCategoryDef, RecipeDef};
use crate::game::{menus, GameSnapshot};
use crate::{Game, GameConfig, GameData};
use fnv_esm::{LoadOrder, Plugin, Schema};
use fnv_script::FunctionTable;
use fnv_sim::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Container, Entry, Item, ItemKind};
use fnv_sim::inventory::Extra;
use fnv_sim::refs::PlacedRef;
use fnv_sim::{clock, FormId, FormStore, Inventory, RefTable, SimConfig, World};

const PLAYER: FormId = FormId::PLAYER_REF;
const INPUT: FormId = FormId(0x100);
const CATALYST: FormId = FormId(0x101);
const OUTPUT: FormId = FormId(0x102);
const STATION_BASE: FormId = FormId(0x200);
const STORAGE: FormId = FormId(0x201);
const CATEGORY: FormId = FormId(0x202);
const OTHER_CATEGORY: FormId = FormId(0x203);
const RECIPE: FormId = FormId(0x204);
const STATION: FormId = FormId(0x300);
const CELL: FormId = FormId(0x400);
const HOUR: FormId = FormId(0x38);
const DAYS: FormId = FormId(0x39);
const HOST_START: u64 = 1_000_000;

struct Fixture { data: GameData, forms: FormStore, refs: RefTable }

fn item() -> Item {
    Item {
        kind: ItemKind::Misc, value: Some(1), money: None, weight: Some(1.0),
        slots: None, projectile: None, health: None, non_playable: false, cant_drop: false,
        armor_class: ArmorClass::Light, power_armor: false, armor_dr: None, armor_dt: None,
        weapon_damage: None, weight_mods: [0.0; 3], drawable: false, not_for_npcs: false,
        thrown: false, quest_item: false, icon: None, placed_count: None, teaches: None,
        cant_be_taken: false, modulates_voice: false, weapon_traits: None,
    }
}

fn fixture() -> Fixture {
    let plugins = vec![Plugin::parse(&common::tes4(), "ProductionTests.esm".into()).unwrap()];
    let lo = LoadOrder::new(&plugins).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let schema = Schema::load_dir(root.join("fnv")).unwrap();
    let functions = root.join("script/functions.json");
    let json = serde_json::from_str(&std::fs::read_to_string(&functions).unwrap()).unwrap();
    let refs = RefTable::default();
    let mut data = GameData::build(&lo, &schema, FunctionTable::load(&functions).unwrap(), &json, &refs);
    let mut forms = FormStore::default();
    for (id, sig, edid, base) in [
        (INPUT, *b"MISC", "TestInput", BaseData::Item(item())),
        (CATALYST, *b"MISC", "TestCatalyst", BaseData::Item(item())),
        (OUTPUT, *b"MISC", "TestOutput", BaseData::Item(item())),
        (STATION_BASE, *b"ACTI", "WorkBench", BaseData::Activator),
        (STORAGE, *b"CONT", "MetalBox01Empty", BaseData::Container(Container { respawns: false, entries: vec![] })),
        (FormId::PLAYER_BASE, *b"NPC_", "Player", BaseData::Actor(ActorBase { entries: vec![], attributes: Some([5; 7]) })),
    ] {
        forms.edids.insert(edid.to_ascii_lowercase(), id);
        forms.sigs.insert(id, sig);
        forms.bases.insert(id, BaseForm { id, sig, edid: Some(edid.into()), name: Some(edid.into()),
            script: None, prompt: None, voice_filter: false, data: base });
        data.edids.insert(edid.to_ascii_lowercase(), id); data.edid_of.insert(id, edid.into());
        data.sigs.insert(id, sig);
    }
    for (id, name) in [(CATEGORY, "WorkbenchRecipes"), (OTHER_CATEGORY, "CampfireRecipes")] {
        forms.sigs.insert(id, *b"RCCT"); // RCCT is game-rule data, absent from the real simulation FormStore index.
        data.edids.insert(name.to_ascii_lowercase(), id); data.edid_of.insert(id, name.into()); data.sigs.insert(id, *b"RCCT");
        data.recipe_categories.insert(id, RecipeCategoryDef { name: name.into(), subcategory: false });
    }
    for (id, name, value) in [
        (FormId(0x35), clock::GAME_YEAR, 2281.0), (FormId(0x36), clock::GAME_MONTH, 9.0),
        (FormId(0x37), clock::GAME_DAY, 18.0), (HOUR, clock::GAME_HOUR, 12.0),
        (DAYS, clock::GAME_DAYS_PASSED, 5.0), (FormId(0x3A), clock::TIME_SCALE, 30.0),
    ] {
        forms.edids.insert(name.to_ascii_lowercase(), id); forms.globals.insert(id, value);
    }
    forms.sigs.insert(CELL, *b"CELL"); data.sigs.insert(CELL, *b"CELL");
    data.cells.insert(CELL, crate::crime::CellDef { owner: None, rank: None, public: true,
        interior: true, off_limits: false, cant_wait: false });
    data.recipes.insert(RECIPE, RecipeDef { form: RECIPE, edid: Some("TestWorkbenchRecipe".into()),
        name: "Synthetic production".into(), conditions: vec![], skill: None,
        category: Some(CATEGORY), subcategory: None,
        ingredients: vec![(INPUT, 2), (CATALYST, 1)], outputs: vec![(OUTPUT, 3)] });
    let mut refs = refs;
    refs.refs.insert(STATION, PlacedRef {
        id: STATION, sig: *b"REFR", edid: None, base: STATION_BASE, cell: Some(CELL), stored_in: None,
        plugin: 0, flags: 0, in_persistent_group: false, pos: [10.0, 0.0, 0.0], rot: [0.0; 3],
        scale: None, count: None, ammo: None, health: None, owner: None, lock: None, teleport: None,
        enable_parent: None, linked: None, patrol_idle: None, patrol_topic: None,
        merchant_container: None, marker_reputation: None, open_by_default: false, primitive: None,
        activate_parents: vec![], parent_activate_only: false, prompt: None, radiation: None,
        radius: None, encounter_zone: None, level_modifier: None,
    });
    refs.by_cell.insert(CELL, vec![STATION]); refs.cell_edids.insert("productiontestcell".into(), CELL);
    Fixture { data, forms, refs }
}

fn bare_game(f: &Fixture) -> Game<'_> {
    let mut world = World::new(&f.forms, &f.refs, SimConfig::default());
    let player = world.get(PLAYER).unwrap(); player.cell = Some(CELL); player.pos = [0.0; 3];
    Game::new(&f.data, world, GameConfig::default())
}

fn game(f: &Fixture) -> Game<'_> {
    let mut g = bare_game(f);
    g.world.add_item(PLAYER, INPUT, 20).unwrap(); g.world.add_item(PLAYER, CATALYST, 10).unwrap();
    g
}

fn cold<'a>(f: &'a Fixture, saved: &GameSnapshot) -> Result<Game<'a>, String> {
    let copy: GameSnapshot = serde_json::from_slice(&serde_json::to_vec(saved).unwrap()).unwrap();
    let world = World::restore(&f.forms, &f.refs, &copy.sim)?;
    Game::restore(&f.data, world, GameConfig::default(), &copy)
}

fn escrow_stock<'g>(g: &'g Game<'_>, id: u64) -> &'g Inventory {
    let escrow = g.production.orders[&id].escrow;
    g.world.peek(escrow).unwrap().inventory.as_ref().unwrap()
}

#[test]
fn queue_recipe_order_moves_actual_inputs_into_one_disabled_reserved_inventory() {
    let f = fixture(); let mut g = game(&f);
    let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    assert_eq!(id, 1); assert_eq!(g.world.item_count(PLAYER, INPUT).unwrap(), 18);
    assert_eq!(g.world.item_count(PLAYER, CATALYST).unwrap(), 9);
    assert_eq!(g.world.item_count(PLAYER, OUTPUT).unwrap(), 0);
    let order = &g.production.orders[&id]; let storage = g.world.peek(order.escrow).unwrap();
    assert_eq!(g.world.inventory_reservation(order.escrow), Some(id));
    assert!(!storage.enabled() && !storage.deleted()); assert_eq!(storage.cell, Some(CELL));
    assert_eq!(storage.extra.owner, Some(FormId::PLAYER_BASE));
    assert_eq!(&order.seal, escrow_stock(&g, id));
    assert_eq!(escrow_stock(&g, id).count(INPUT), 2); assert_eq!(escrow_stock(&g, id).count(CATALYST), 1);
    assert_eq!(order.state, OrderState::Working);
    assert_eq!(order.deadline, WorkDeadline::GameHours { started: "0".into(), due: GAME_HOURS_PER_BATCH.to_string() });
}

#[test]
fn repeating_the_same_trusted_command_id_does_not_spend_or_allocate_twice() {
    let f = fixture(); let mut g = game(&f);
    g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1, WorkTiming::GameHours(0.25)).unwrap();
    let before = g.snapshot(); let serial = g.world.presence_serial();
    for _ in 0..3 {
        assert_eq!(g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1, WorkTiming::GameHours(0.25)).unwrap(), 1);
    }
    assert_eq!(g.snapshot(), before); assert_eq!(g.world.presence_serial(), serial);
}

#[test]
fn a_conflicting_reused_command_id_is_rejected_without_any_change() {
    let f = fixture(); let mut g = game(&f);
    g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap(); let before = g.snapshot();
    assert!(g.queue_production_order(STATION, CATEGORY, RECIPE, 2, 1, WorkTiming::GameHours(0.25)).is_err());
    assert!(g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1, WorkTiming::HostUtc { now: HOST_START, seconds: 60 }).is_err());
    assert_eq!(g.snapshot(), before);
}

#[test]
fn funded_real_escrow_denies_ordinary_inventory_and_delete_calls() {
    let f = fixture(); let mut g = game(&f);
    let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap(); let escrow = g.production.orders[&id].escrow;
    let before = g.snapshot();
    assert!(matches!(g.world.inventory_mut(escrow), Err(fnv_sim::world::SimError::ReservedInventory(e)) if e == escrow));
    assert_eq!(g.world.remove_item(escrow, INPUT, 1), Err(fnv_sim::world::SimError::ReservedInventory(escrow)));
    assert_eq!(g.world.set_deleted(escrow, true), Err(fnv_sim::world::SimError::ReservedInventory(escrow)));
    assert_eq!(g.snapshot(), before);
}

#[test]
fn accepted_wait_hours_complete_once_and_outputs_remain_real_stock_until_collection() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.world.advance_hours(0.125).unwrap(); assert!(g.advance_production_orders(None).is_empty());
    assert_eq!(g.production.orders[&id].state, OrderState::Working);
    g.world.advance_hours(0.125).unwrap(); assert_eq!(g.advance_production_orders(None).len(), 1);
    assert_eq!(g.production.orders[&id].state, OrderState::Ready);
    assert_eq!(escrow_stock(&g, id).count(INPUT), 0); assert_eq!(escrow_stock(&g, id).count(OUTPUT), 3);
    assert_eq!(g.world.item_count(PLAYER, OUTPUT).unwrap(), 0);
    assert_eq!(g.misc_stat(crate::misc_stats::ITEMS_CRAFTED), 1.0);
    let before = g.snapshot(); assert!(g.advance_production_orders(None).is_empty());
    assert_eq!(g.snapshot(), before);
    assert_eq!(g.collect_production_order(id).unwrap(), [(OUTPUT, 3)]);
    assert_eq!(g.world.item_count(PLAYER, OUTPUT).unwrap(), 3);
}

#[test]
fn paused_game_ticks_never_mint_accepted_time_or_finish_work() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.cfg.frame_seconds = 3600.0; g.open_menu(menus::START); assert!(g.paused());
    let stock = escrow_stock(&g, id).clone(); let hours = g.world.accepted_game_hours();
    for _ in 0..3 { g.tick(); }
    assert_eq!(g.world.accepted_game_hours(), hours); assert_eq!(g.production.orders[&id].state, OrderState::Working);
    assert_eq!(escrow_stock(&g, id), &stock);
}

#[test]
fn direct_calendar_jumps_and_host_samples_cannot_finish_a_game_hours_order() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.world.set_global(HOUR, 23.0).unwrap(); g.world.set_global(DAYS, 9999.0).unwrap();
    assert!(g.advance_production_orders(Some(u64::MAX)).is_empty());
    assert_eq!(g.world.accepted_game_hours(), Some(0.0)); assert_eq!(g.production.orders[&id].state, OrderState::Working);
}

#[test]
fn cancelling_returns_the_same_owned_condition_mod_groups_without_double_refund() {
    let f = fixture(); let mut g = bare_game(&f);
    let extra = Extra { condition: Some(0.42), owner: Some(FormId(0x999)), equipped: true, mods: 5 };
    g.world.add_item_extra(PLAYER, INPUT, 2, extra).unwrap(); g.world.add_item(PLAYER, CATALYST, 1).unwrap();
    let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap(); let escrow = g.production.orders[&id].escrow;
    assert_eq!(g.cancel_production_order(id).unwrap(), [(INPUT, 2), (CATALYST, 1)]);
    assert_eq!(g.production.orders[&id].state, OrderState::Cancelled);
    assert_eq!(g.world.inventory_reservation(escrow), None); assert!(g.world.peek(escrow).unwrap().deleted());
    assert!(g.world.peek(PLAYER).unwrap().inventory.as_ref().unwrap().groups().any(|group|
        group == (INPUT, Extra { equipped: false, ..extra }, 2)));
    let before = g.snapshot(); assert!(g.cancel_production_order(id).unwrap().is_empty());
    assert!(g.collect_production_order(id).is_err()); assert_eq!(g.snapshot(), before);
}

#[test]
fn collection_capacity_overflow_keeps_ready_outputs_reservation_and_player_exact() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.world.advance_hours(0.25).unwrap(); g.advance_production_orders(None);
    g.world.add_item(PLAYER, OUTPUT, i32::MAX).unwrap(); let before = g.snapshot();
    let serial = g.world.presence_serial(); assert!(g.collect_production_order(id).is_err());
    assert_eq!(g.snapshot(), before); assert_eq!(g.world.presence_serial(), serial);
    assert_eq!(escrow_stock(&g, id).count(OUTPUT), 3);
}

#[test]
fn working_order_cold_save_restores_real_escrow_and_completes_from_remaining_time() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.world.advance_hours(0.125).unwrap(); let saved = g.snapshot();
    let mut restored = cold(&f, &saved).unwrap(); assert_eq!(restored.production, saved.production);
    assert_eq!(restored.world.snapshot(), saved.sim); assert_eq!(restored.production.orders[&id].state, OrderState::Working);
    assert!(restored.advance_production_orders(None).is_empty());
    restored.world.advance_hours(0.125).unwrap(); restored.advance_production_orders(None);
    assert_eq!(restored.production.orders[&id].state, OrderState::Ready); assert_eq!(escrow_stock(&restored, id).count(OUTPUT), 3);
}

#[test]
fn ready_and_collected_cold_saves_neither_rerun_production_nor_duplicate_delivery() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.world.advance_hours(0.25).unwrap(); g.advance_production_orders(None); let saved = g.snapshot();
    let mut ready = cold(&f, &saved).unwrap(); assert_eq!(ready.production.orders[&id].state, OrderState::Ready);
    assert!(ready.advance_production_orders(None).is_empty()); assert_eq!(escrow_stock(&ready, id).count(OUTPUT), 3);
    assert_eq!(ready.collect_production_order(id).unwrap(), [(OUTPUT, 3)]);
    let saved = ready.snapshot(); let mut closed = cold(&f, &saved).unwrap();
    assert_eq!(closed.production.orders[&id].state, OrderState::Collected);
    let before = closed.snapshot(); assert!(closed.collect_production_order(id).unwrap().is_empty());
    assert_eq!(closed.world.item_count(PLAYER, OUTPUT).unwrap(), 3); assert_eq!(closed.snapshot(), before);
}

#[test]
fn cancelled_cold_save_retains_command_history_without_recreating_inputs() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    g.cancel_production_order(id).unwrap(); let saved = g.snapshot(); let mut restored = cold(&f, &saved).unwrap();
    assert_eq!(restored.production.orders[&id].state, OrderState::Cancelled); let before = restored.snapshot();
    assert!(restored.cancel_production_order(id).unwrap().is_empty());
    assert_eq!(restored.queue_production_order(STATION, CATEGORY, RECIPE, 1, id, WorkTiming::GameHours(0.25)).unwrap(), id);
    assert_eq!(restored.snapshot(), before); assert_eq!(restored.world.item_count(PLAYER, INPUT).unwrap(), 20);
}

#[test]
fn changed_recipe_on_cold_restore_fails_closed_against_the_actual_saved_world() {
    let mut f = fixture(); let saved = {
        let mut g = game(&f); g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap(); g.snapshot()
    };
    f.data.recipes.get_mut(&RECIPE).unwrap().outputs[0].1 = 4;
    assert!(cold(&f, &saved).is_err());
    let world = World::restore(&f.forms, &f.refs, &saved.sim).unwrap();
    assert_eq!(world.snapshot(), saved.sim, "failed game binding must not rewrite reserved inputs");
}

#[test]
fn changed_actual_escrow_stock_is_held_at_runtime_and_rejected_on_cold_restore() {
    let f = fixture(); let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
    let escrow = g.production.orders[&id].escrow;
    // Deliberate trusted-engine corruption, bypassing the ordinary reserved API:
    // the public read-only seal must detect it, not overwrite real stock to match.
    g.world.get(escrow).unwrap().inventory.as_mut().unwrap().add(INPUT, 1);
    g.world.advance_hours(0.25).unwrap(); let before = g.snapshot();
    let notices = g.advance_production_orders(None); assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("held")); assert_eq!(g.production.orders[&id].state, OrderState::Working);
    assert_eq!(g.snapshot(), before); assert!(cold(&f, &before).is_err());
}

#[test]
fn host_48_hours_is_exactly_172800_seconds_and_offline_restart_finishes_once() {
    assert_eq!(TWO_DAYS_SECONDS, 172800);
    let f = fixture(); let mut g = game(&f);
    let id = g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1,
        WorkTiming::HostUtc { now: HOST_START, seconds: TWO_DAYS_SECONDS }).unwrap();
    assert_eq!(g.production.orders[&id].deadline,
        WorkDeadline::HostUtc { started: HOST_START, due: HOST_START + 172800 });
    let saved = g.snapshot(); let mut offline = cold(&f, &saved).unwrap();
    assert!(offline.advance_production_orders(Some(HOST_START + 172799)).is_empty());
    assert_eq!(offline.production.orders[&id].state, OrderState::Working);
    let saved = offline.snapshot(); let mut restarted = cold(&f, &saved).unwrap();
    assert_eq!(restarted.advance_production_orders(Some(HOST_START + 172800)).len(), 1);
    assert_eq!(restarted.production.orders[&id].state, OrderState::Ready);
    assert_eq!(restarted.world.accepted_game_hours(), Some(0.0)); assert_eq!(escrow_stock(&restarted, id).count(OUTPUT), 3);
    let saved = restarted.snapshot(); let mut ready = cold(&f, &saved).unwrap(); let before = ready.snapshot();
    assert!(ready.advance_production_orders(Some(HOST_START + 500000)).is_empty());
    assert_eq!(ready.snapshot(), before); assert_eq!(ready.misc_stat(crate::misc_stats::ITEMS_CRAFTED), 1.0);
}

#[test]
fn backwards_host_sample_after_cold_restart_cannot_undo_elapsed_host_time() {
    let f = fixture(); let mut g = game(&f);
    let id = g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1,
        WorkTiming::HostUtc { now: HOST_START, seconds: TWO_DAYS_SECONDS }).unwrap();
    g.advance_production_orders(Some(HOST_START + 60)); let saved = g.snapshot();
    let mut restarted = cold(&f, &saved).unwrap(); let before = restarted.snapshot();
    assert!(restarted.advance_production_orders(Some(HOST_START - 1000)).is_empty());
    assert_eq!(restarted.production.last_host_utc, Some(HOST_START + 60));
    assert_eq!(restarted.production.orders[&id].state, OrderState::Working); assert_eq!(restarted.snapshot(), before);
}

#[test]
fn accepted_game_time_cannot_complete_a_host_clock_order_without_host_sample() {
    let f = fixture(); let mut g = game(&f);
    let id = g.queue_production_order(STATION, CATEGORY, RECIPE, 1, 1,
        WorkTiming::HostUtc { now: HOST_START, seconds: TWO_DAYS_SECONDS }).unwrap();
    g.world.advance_hours(48.0).unwrap(); assert!(g.advance_production_orders(None).is_empty());
    assert_eq!(g.production.orders[&id].state, OrderState::Working);
}

#[test]
fn arbitrary_activator_or_wrong_station_recipe_category_is_not_admitted() {
    for arbitrary_station in [true, false] {
        let mut f = fixture();
        if arbitrary_station { f.forms.bases.get_mut(&STATION_BASE).unwrap().edid = Some("UnrelatedActivator".into()); }
        else { f.data.recipes.get_mut(&RECIPE).unwrap().category = Some(OTHER_CATEGORY); }
        let mut g = game(&f); let before = g.snapshot();
        let category = if arbitrary_station { CATEGORY } else { OTHER_CATEGORY };
        assert!(g.queue_recipe_order(STATION, category, RECIPE, 1).is_err()); assert_eq!(g.snapshot(), before);
    }
}

#[test]
fn insufficient_second_ingredient_and_out_of_reach_admission_leave_no_escrow() {
    let f = fixture(); let mut g = bare_game(&f); g.world.add_item(PLAYER, INPUT, 20).unwrap();
    let before = g.snapshot(); let serial = g.world.presence_serial();
    assert!(g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).is_err());
    assert_eq!(g.snapshot(), before); assert_eq!(g.world.presence_serial(), serial);
    g.world.add_item(PLAYER, CATALYST, 10).unwrap(); g.world.get(PLAYER).unwrap().pos = [5000.0, 0.0, 0.0];
    let before = g.snapshot(); assert!(g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).is_err()); assert_eq!(g.snapshot(), before);
}

#[test]
fn newly_nonempty_storage_base_is_rejected_on_cold_restore_without_rewriting_old_stock() {
    let mut f = fixture(); let saved = {
        let mut g = game(&f); g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap(); g.snapshot()
    };
    let BaseData::Container(container) = &mut f.forms.bases.get_mut(&STORAGE).unwrap().data else { unreachable!() };
    container.entries.push(Entry::new(INPUT, 1));
    let world = World::restore(&f.forms, &f.refs, &saved.sim).unwrap();
    assert_eq!(world.snapshot(), saved.sim, "restoring the World must not inject newly authored stock into an existing escrow");
    assert!(Game::restore(&f.data, world, GameConfig::default(), &saved).is_err(),
        "matching book quantities alone do not establish a safe empty storage base");
}

#[test]
fn changed_station_access_or_identity_blocks_refund_and_collection_without_mutation() {
    let f = fixture();
    for case in 0..4 {
        let mut g = game(&f); let id = g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).unwrap();
        let ready = case % 2 == 1;
        if ready { g.world.advance_hours(0.25).unwrap(); g.advance_production_orders(None); }
        match case {
            0 => g.world.lock(STATION, Some(50)).unwrap(),
            1 => { g.world.get(STATION).unwrap().extra.owner = Some(FormId(0x999)); }
            2 => { g.world.get(STATION).unwrap().cell = Some(FormId(0x999)); }
            3 => { g.world.get(STATION).unwrap().base = INPUT; }
            _ => unreachable!(),
        }
        let before = g.snapshot(); let serial = g.world.presence_serial();
        let result = if ready { g.collect_production_order(id) } else { g.cancel_production_order(id) };
        assert!(result.is_err(), "access/identity case {case}");
        assert_eq!(g.snapshot(), before); assert_eq!(g.world.presence_serial(), serial);
        if case >= 2 { assert!(cold(&f, &before).is_err(), "saved station identity changed"); }
    }
}

#[test]
fn nonfinite_input_condition_is_rejected_before_any_real_stock_deposit() {
    let f = fixture();
    for condition in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut g = game(&f);
        // There is already enough plain INPUT for the recipe. Even an unselected
        // malformed group of that same input form must not admit an unusable seal.
        g.world.add_item_extra(PLAYER, INPUT, 1, Extra {
            condition: Some(condition), owner: Some(FormId(0x999)), equipped: false, mods: 3,
        }).unwrap();
        let groups = |g: &Game<'_>| g.world.peek(PLAYER).unwrap().inventory.as_ref().unwrap()
            .groups().map(|(item, extra, count)|
                (item, extra.condition.map(f32::to_bits), extra.owner, extra.equipped, extra.mods, count))
            .collect::<Vec<_>>();
        let before_groups = groups(&g);
        let before = g.world.snapshot(); let book = g.production.clone(); let serial = g.world.presence_serial();
        let refs: Vec<_> = g.world.changed().map(|(id, _)| id).collect();

        assert!(g.queue_recipe_order(STATION, CATEGORY, RECIPE, 1).is_err(), "must refuse {condition:?}");

        assert_eq!(groups(&g), before_groups, "compare condition bits: NaN cannot use ordinary PartialEq");
        assert!(g.world.peek(PLAYER).unwrap().inventory.as_ref().unwrap().groups().any(|(item, extra, count)|
            item == INPUT && count == 1 && extra.condition.is_some_and(|value|
                if condition.is_nan() { value.is_nan() } else { value == condition })));
        assert_eq!(g.world.item_count(PLAYER, INPUT).unwrap(), 21);
        assert_eq!(g.world.item_count(PLAYER, CATALYST).unwrap(), 10);
        assert_eq!(g.world.item_count(PLAYER, OUTPUT).unwrap(), 0);
        let after = g.world.snapshot();
        assert_eq!(after.rng_word_pos, before.rng_word_pos);
        assert_eq!(after.next_dynamic, before.next_dynamic);
        assert_eq!(after.globals, before.globals);
        assert_eq!(after.accepted_game_hours, before.accepted_game_hours);
        assert_eq!(after.inventory_reservations, before.inventory_reservations);
        assert!(after.inventory_reservations.is_empty());
        assert_eq!(g.world.changed().map(|(id, _)| id).collect::<Vec<_>>(), refs);
        assert_eq!(g.world.presence_serial(), serial);
        assert_eq!(g.production, book);
        assert!(g.production.orders.is_empty(), "no escrow or order may be created");
    }
}
