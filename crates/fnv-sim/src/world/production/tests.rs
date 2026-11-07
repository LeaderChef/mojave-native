//! Real World transactions over declared synthetic forms; no original game data.
//! The ordinary add/remove APIs are the oracle for leveled-roll order, not a copy
//! of the production algorithm. Cargo execution belongs to the coordinating agent.

use super::InventoryExchangeError;
use crate::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Entry, Item, ItemKind, LeveledEntry, LeveledList};
use crate::inventory::Extra;
use crate::refs::PlacedRef;
use crate::world::{SimConfig, SimError, Snapshot, World};
use crate::{FormId, FormStore, RefTable};
use rand_chacha::ChaCha8Rng;

const PLAYER: FormId = FormId::PLAYER_REF;
const INPUT: FormId = FormId(0x100);
const PRODUCT: FormId = FormId(0x101);
const GEAR: FormId = FormId(0x102);
const ALTERNATE: FormId = FormId(0x103);
const CHOICE: FormId = FormId(0x110);
const NOTHING: FormId = FormId(0x111);
const CYCLE: FormId = FormId(0x112);
const BAD_LEAF: FormId = FormId(0x113);
const UNKNOWN: FormId = FormId(0xFFFF);
const NPC_BASE: FormId = FormId(0x200);
const NPC: FormId = FormId(0x300);

struct Fixture {
    forms: FormStore,
    refs: RefTable,
}

fn base(id: FormId, sig: [u8; 4], data: BaseData) -> BaseForm {
    BaseForm { id, sig, edid: None, name: None, script: None, prompt: None, voice_filter: false, data }
}

fn item(kind: ItemKind) -> Item {
    Item {
        kind,
        value: Some(1),
        money: None,
        weight: Some(1.0),
        slots: None,
        projectile: None,
        health: Some(100),
        non_playable: false,
        cant_drop: false,
        armor_class: ArmorClass::Light,
        power_armor: false,
        armor_dr: None,
        armor_dt: None,
        weapon_damage: None,
        weight_mods: [0.0; 3],
        drawable: false,
        not_for_npcs: false,
        thrown: false,
        quest_item: false,
        icon: None,
        placed_count: None,
        teaches: None,
        cant_be_taken: false,
        modulates_voice: false,
        weapon_traits: None,
    }
}

fn list(chance_none: u8, use_all: bool, entries: &[(FormId, u16)]) -> BaseData {
    BaseData::LeveledItem(LeveledList {
        chance_none,
        all_levels: true,
        each: false,
        use_all,
        global: None,
        entries: entries.iter().map(|&(item, count)| LeveledEntry { level: 1, item, count, condition: None }).collect(),
    })
}

fn fixture() -> Fixture {
    let mut forms = FormStore::default();
    for (id, sig, data) in [
        (INPUT, *b"MISC", BaseData::Item(item(ItemKind::Misc))),
        (PRODUCT, *b"MISC", BaseData::Item(item(ItemKind::Misc))),
        (GEAR, *b"WEAP", BaseData::Item(item(ItemKind::Weapon))),
        (ALTERNATE, *b"MISC", BaseData::Item(item(ItemKind::Misc))),
        (CHOICE, *b"LVLI", list(0, false, &[(PRODUCT, 1), (ALTERNATE, 2)])),
        (NOTHING, *b"LVLI", list(100, false, &[(PRODUCT, 1)])),
        (CYCLE, *b"LVLI", list(0, false, &[(CYCLE, 1)])),
        // Resolves a valid entry before the unknown leaf; the primitive must reject
        // all of it rather than commit the first candidate output.
        (BAD_LEAF, *b"LVLI", list(0, true, &[(PRODUCT, 1), (UNKNOWN, 1)])),
        (FormId::PLAYER_BASE, *b"NPC_", BaseData::Actor(ActorBase { entries: Vec::new(), attributes: Some([5; 7]) })),
        (NPC_BASE, *b"NPC_", BaseData::Actor(ActorBase { entries: vec![Entry::new(INPUT, 4), Entry::new(CHOICE, 1)], attributes: Some([5; 7]) })),
    ] {
        forms.sigs.insert(id, sig);
        forms.bases.insert(id, base(id, sig, data));
    }
    let mut refs = RefTable::default();
    refs.refs.insert(NPC, PlacedRef {
        id: NPC,
        sig: *b"ACHR",
        edid: None,
        base: NPC_BASE,
        cell: None,
        stored_in: None,
        plugin: 0,
        flags: 0,
        in_persistent_group: false,
        pos: [0.0; 3],
        rot: [0.0; 3],
        scale: None,
        count: None,
        ammo: None,
        health: None,
        owner: None,
        lock: None,
        teleport: None,
        enable_parent: None,
        linked: None,
        patrol_idle: None,
        patrol_topic: None,
        merchant_container: None,
        marker_reputation: None,
        open_by_default: false,
        primitive: None,
        activate_parents: Vec::new(),
        parent_activate_only: false,
        prompt: None,
        radiation: None,
        radius: None,
        encounter_zone: None,
        level_modifier: None,
    });
    Fixture { forms, refs }
}

fn world(f: &Fixture) -> World<'_> {
    World::new(&f.forms, &f.refs, SimConfig::default())
}

fn assert_unchanged(w: &World<'_>, before: &Snapshot, rng: &ChaCha8Rng, presence: u64) {
    assert_eq!(&w.snapshot(), before, "inventory, references, owed items and saved RNG must be identical");
    assert_eq!(&w.rng, rng, "the full ChaCha state must not advance on failure");
    assert_eq!(w.presence_serial(), presence, "failed materialization must not leak a presence change");
}

#[test]
fn exact_exchange_consumes_plain_first_and_preserves_remaining_extra_groups() {
    let f = fixture();
    let mut w = world(&f);
    let owned = Extra { condition: Some(0.4), owner: Some(FormId(0x888)), equipped: true, mods: 3 };
    let untouched = Extra { condition: Some(0.75), owner: Some(FormId(0x889)), equipped: true, mods: 5 };
    w.add_item(PLAYER, INPUT, 3).unwrap();
    w.add_item_extra(PLAYER, INPUT, 2, owned).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, untouched).unwrap();
    w.get(PLAYER).unwrap().scale = Some(1.5);
    let before_rng = w.rng.clone();

    let added = w.exchange_inventory(PLAYER, &[(INPUT, 4)], &[(PRODUCT, 2)]).unwrap();

    assert_eq!(added, [(PRODUCT, 2)]);
    let inventory = w.peek(PLAYER).unwrap().inventory.as_ref().unwrap();
    assert_eq!(inventory.groups().filter(|g| g.0 == INPUT).collect::<Vec<_>>(), [(INPUT, owned, 1)]);
    assert_eq!(inventory.groups().filter(|g| g.0 == GEAR).collect::<Vec<_>>(), [(GEAR, untouched, 1)]);
    assert_eq!(inventory.groups().filter(|g| g.0 == PRODUCT).collect::<Vec<_>>(), [(PRODUCT, Extra::default(), 2)]);
    assert_eq!(w.peek(PLAYER).unwrap().scale, Some(1.5));
    assert_eq!(w.rng, before_rng, "plain outputs consume no random roll");
}

#[test]
fn duplicate_ingredients_require_their_total_and_failed_exchange_changes_nothing() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 1).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, 1), (INPUT, 1)], &[(PRODUCT, 1)]),
        Err(InventoryExchangeError::Missing { item: INPUT, have: 1, need: 2 }));
    assert_unchanged(&w, &before, &rng, presence);

    w.add_item(PLAYER, INPUT, 1).unwrap();
    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, 1), (INPUT, 1)], &[(PRODUCT, 1)]).unwrap(), [(PRODUCT, 1)]);
    assert_eq!(w.item_count(PLAYER, INPUT).unwrap(), 0);
    assert_eq!(w.item_count(PLAYER, PRODUCT).unwrap(), 1);
}

#[test]
fn invalid_second_output_is_refused_before_consuming_a_valid_ingredient() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert!(matches!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(PRODUCT, 1), (UNKNOWN, 1)]),
        Err(InventoryExchangeError::Sim(SimError::UnknownForm(id))) if id == UNKNOWN));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn output_overflow_across_condition_groups_rolls_back_all_candidate_outputs() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    w.add_item_extra(PLAYER, GEAR, i32::MAX - 2, Extra { condition: Some(0.5), ..Extra::default() }).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, Extra { condition: Some(0.8), ..Extra::default() }).unwrap();
    assert_eq!(w.item_count(PLAYER, GEAR).unwrap(), i32::MAX - 1);
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(PRODUCT, 1), (GEAR, 2)]),
        Err(InventoryExchangeError::CountOverflow(GEAR)));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn consuming_and_producing_the_same_item_checks_the_post_consumption_capacity() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, i32::MAX).unwrap();
    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, 3)], &[(INPUT, 2)]).unwrap(), [(INPUT, 2)]);
    assert_eq!(w.item_count(PLAYER, INPUT).unwrap(), i32::MAX - 1);
}

#[test]
fn separate_leveled_output_rows_match_two_ordinary_adds_and_their_rng() {
    let f = fixture();
    for seed in 0..16 {
        let mut w = World::new(&f.forms, &f.refs, SimConfig { seed, ..SimConfig::default() });
        w.add_item(PLAYER, INPUT, 2).unwrap();
        let mut ordinary = World::restore(&f.forms, &f.refs, &w.snapshot()).unwrap();
        assert_eq!(ordinary.remove_item(PLAYER, INPUT, 1).unwrap(), 1);
        let mut expected = ordinary.add_item(PLAYER, CHOICE, 1).unwrap();
        expected.extend(ordinary.add_item(PLAYER, CHOICE, 1).unwrap());

        let added = w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(CHOICE, 1), (CHOICE, 1)]).unwrap();

        assert_eq!(added, expected, "output row order for seed {seed}");
        assert_eq!(w.snapshot(), ordinary.snapshot(), "actual inventory and RNG must match the ordinary APIs for seed {seed}");
        assert_eq!(w.rng, ordinary.rng);
    }
}

#[test]
fn chance_none_is_a_successful_zero_output_and_commits_its_random_roll() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before_rng = w.rng.clone();
    let mut ordinary = World::restore(&f.forms, &f.refs, &w.snapshot()).unwrap();
    ordinary.remove_item(PLAYER, INPUT, 1).unwrap();
    assert!(ordinary.add_item(PLAYER, NOTHING, 3).unwrap().is_empty());

    assert!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(NOTHING, 3)]).unwrap().is_empty());
    assert_eq!(w.item_count(PLAYER, INPUT).unwrap(), 1);
    assert_eq!(w.item_count(PLAYER, PRODUCT).unwrap(), 0);
    assert_ne!(w.rng, before_rng, "a successful no-output roll is still a committed roll");
    assert_eq!(w.snapshot(), ordinary.snapshot());
}

#[test]
fn unknown_nested_leaf_rolls_back_prior_output_and_both_rng_stream_steps() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert!(matches!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(CHOICE, 1), (BAD_LEAF, 1)]),
        Err(InventoryExchangeError::Sim(SimError::UnknownForm(id))) if id == UNKNOWN));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn cyclic_second_list_rolls_back_prior_resolved_output_and_rng() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(CHOICE, 1), (CYCLE, 1)]).is_err());
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn failed_first_npc_exchange_restores_owed_death_items_reference_and_rng() {
    let f = fixture();
    let mut w = world(&f);
    assert_eq!(w.add_death_item(NPC, CHOICE).unwrap(), None);
    assert!(w.peek(NPC).is_none());
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    // Prove this fixture really materializes RNG-dependent base/death inventories.
    let mut control = World::restore(&f.forms, &f.refs, &before).unwrap();
    control.get(NPC).unwrap();
    assert_ne!(control.rng, rng);
    assert!(!control.owed_death_items().contains_key(&NPC));

    assert!(w.exchange_inventory(NPC, &[(INPUT, 1)], &[(CHOICE, 1), (CYCLE, 1)]).is_err());

    assert!(w.peek(NPC).is_none());
    assert_eq!(w.owed_death_items().get(&NPC), Some(&CHOICE));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn a_cycle_during_initial_death_inventory_restores_the_removed_owed_item() {
    let f = fixture();
    let mut w = world(&f);
    assert_eq!(w.add_death_item(NPC, CYCLE).unwrap(), None);
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    assert!(w.exchange_inventory(NPC, &[], &[(PRODUCT, 1)]).is_err());

    assert!(w.peek(NPC).is_none());
    assert_eq!(w.owed_death_items().get(&NPC), Some(&CYCLE));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn first_player_materialization_failure_restores_the_actual_presence_serial_change() {
    let mut f = fixture();
    let BaseData::Actor(player_base) = &mut f.forms.bases.get_mut(&FormId::PLAYER_BASE).unwrap().data else { panic!("synthetic player base") };
    player_base.entries.push(Entry::new(CHOICE, 1));
    let mut w = world(&f);
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    let mut control = World::restore(&f.forms, &f.refs, &before).unwrap();
    let control_presence = control.presence_serial();
    control.get(PLAYER).unwrap();
    assert_ne!(control.presence_serial(), control_presence, "unplaced player materialization really changes presence");
    assert_ne!(control.rng, rng);

    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(PRODUCT, 1)]),
        Err(InventoryExchangeError::Missing { item: INPUT, have: 0, need: 1 }));

    assert!(w.peek(PLAYER).is_none());
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn invalid_and_overflowing_requests_do_not_materialize_or_advance_a_world() {
    let f = fixture();
    let mut w = world(&f);
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    assert_eq!(w.exchange_inventory(PLAYER, &[(INPUT, i32::MAX), (INPUT, 1)], &[(PRODUCT, 1)]),
        Err(InventoryExchangeError::CountOverflow(INPUT)));
    assert_unchanged(&w, &before, &rng, presence);
    for bad in [0, -1] {
        assert!(matches!(w.exchange_inventory(PLAYER, &[(INPUT, bad)], &[(PRODUCT, 1)]),
            Err(InventoryExchangeError::Sim(SimError::BadCount(n))) if n == bad));
        assert_unchanged(&w, &before, &rng, presence);
        assert!(matches!(w.exchange_inventory(PLAYER, &[], &[(PRODUCT, bad)]),
            Err(InventoryExchangeError::Sim(SimError::BadCount(n))) if n == bad));
        assert_unchanged(&w, &before, &rng, presence);
    }
    assert!(w.peek(PLAYER).is_none());
}

#[test]
fn an_each_list_roll_limit_refuses_production_instead_of_consuming_a_truncated_batch() {
    let mut f = fixture();
    let each_list = FormId(0x114);
    let mut data = list(0, true, &[(PRODUCT, 1)]);
    let BaseData::LeveledItem(lvli) = &mut data else { panic!("synthetic leveled list") };
    lvli.each = true;
    f.forms.sigs.insert(each_list, *b"LVLI");
    f.forms.bases.insert(each_list, base(each_list, *b"LVLI", data));
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 10_001).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    // The old, general AddItem cap remains its policy. Production must not treat
    // that policy as an exact 10,001-unit exchange and silently lose an input.
    let mut ordinary = World::restore(&f.forms, &f.refs, &before).unwrap();
    ordinary.add_item(PLAYER, each_list, 10_001).unwrap();
    assert_eq!(ordinary.item_count(PLAYER, PRODUCT).unwrap(), 10_000);

    assert!(w.exchange_inventory(PLAYER, &[(INPUT, 10_001)], &[(CHOICE, 1), (each_list, 10_001)]).is_err(),
        "a resolver roll cap must fail the entire production exchange");
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn a_nested_leveled_multiplier_overflow_refuses_production_without_any_effect() {
    let mut f = fixture();
    let outer = FormId(0x115);
    let middle = FormId(0x116);
    let inner = FormId(0x117);
    for (id, target, count) in [(outer, middle, u16::MAX), (middle, inner, u16::MAX), (inner, GEAR, 1)] {
        f.forms.sigs.insert(id, *b"LVLI");
        f.forms.bases.insert(id, base(id, *b"LVLI", list(0, true, &[(target, count)])));
    }
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 1).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();

    // Every authored row fits its type, but 65,535 * 65,535 does not fit i32.
    // General AddItem currently saturates this nested multiplier; the exact
    // production path must reject it before consuming or committing any row.
    let mut ordinary = World::restore(&f.forms, &f.refs, &before).unwrap();
    assert_eq!(ordinary.add_item(PLAYER, outer, 1).unwrap(), [(GEAR, i32::MAX)]);

    assert!(w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(CHOICE, 1), (outer, 1)]).is_err(),
        "a nested resolver multiplication must not silently saturate a production yield");
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn nested_use_all_work_exhaustion_refuses_the_whole_exchange() {
    let mut f = fixture();
    // No cycle, oversized row or overflowing yield: five levels of ten children
    // have 100,000 leaf visits. The production work budget must bound the whole
    // traversal, not merely each list's count or the depth of an individual path.
    for depth in 0..5 {
        let id = FormId(0x180 + depth);
        let target = if depth == 4 { PRODUCT } else { FormId(id.0 + 1) };
        f.forms.sigs.insert(id, *b"LVLI");
        f.forms.bases.insert(id, base(id, *b"LVLI", list(0, true, &[(target, 1); 10])));
    }
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    let result = w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(CHOICE, 1), (FormId(0x180), 1)]);
    assert!(matches!(result, Err(InventoryExchangeError::Sim(SimError::Leveled(crate::leveled::LeveledError::WorkLimit(_))))));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn nested_empty_each_rolls_still_spend_the_production_work_budget() {
    let mut f = fixture();
    let empty_each = FormId(0x190);
    let outer = FormId(0x191);
    let mut data = list(100, true, &[(PRODUCT, 1)]);
    let BaseData::LeveledItem(lvli) = &mut data else { panic!("synthetic list") };
    lvli.each = true;
    f.forms.sigs.insert(empty_each, *b"LVLI");
    f.forms.bases.insert(empty_each, base(empty_each, *b"LVLI", data));
    f.forms.sigs.insert(outer, *b"LVLI");
    f.forms.bases.insert(outer, base(outer, *b"LVLI", list(0, true, &[(empty_each, 10_000); 10])));
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    // Only eleven list nodes, but 100,000 valid each-rolls. A node-only budget
    // would accept and consume input after doing work far beyond its stated bound.
    let result = w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(outer, 1)]);
    assert!(matches!(result, Err(InventoryExchangeError::Sim(SimError::Leveled(crate::leveled::LeveledError::WorkLimit(_))))));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn zero_count_entries_still_spend_the_entry_scan_budget() {
    let mut f = fixture();
    let oversized = FormId(0x192);
    // Child calls with count zero return early. Counting child visits alone cannot
    // bound scanning/copying this list, even though it produces no units at all.
    let entries = vec![(PRODUCT, 0); 100_000];
    f.forms.sigs.insert(oversized, *b"LVLI");
    f.forms.bases.insert(oversized, base(oversized, *b"LVLI", list(0, true, &entries)));
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    let result = w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(oversized, 1)]);
    assert!(matches!(result, Err(InventoryExchangeError::Sim(SimError::Leveled(crate::leveled::LeveledError::WorkLimit(_))))));
    assert_unchanged(&w, &before, &rng, presence);
}

#[test]
fn repeated_zero_count_children_spend_work_even_when_their_body_returns_early() {
    let mut f = fixture();
    let each = FormId(0x193);
    let mut data = list(0, true, &[(PRODUCT, 0); 100]);
    let BaseData::LeveledItem(lvli) = &mut data else { panic!("synthetic list") };
    lvli.each = true;
    f.forms.sigs.insert(each, *b"LVLI");
    f.forms.bases.insert(each, base(each, *b"LVLI", data));
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    let before = w.snapshot();
    let rng = w.rng.clone();
    let presence = w.presence_serial();
    // One short entry scan and 1,000 rolls, but 100,000 zero-child iterations.
    // A child that returns early is still work and must not be a free iteration.
    let result = w.exchange_inventory(PLAYER, &[(INPUT, 1)], &[(each, 1_000)]);
    assert!(matches!(result, Err(InventoryExchangeError::Sim(SimError::Leveled(crate::leveled::LeveledError::WorkLimit(_))))));
    assert_unchanged(&w, &before, &rng, presence);
}
