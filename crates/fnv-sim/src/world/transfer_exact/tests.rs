//! Real World calls over synthetic forms; no game assets or copied resolver.

use super::InventoryTransferError;
use crate::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Entry, Item, ItemKind, LeveledEntry, LeveledList};
use crate::inventory::{Extra, Pick};
use crate::refs::PlacedRef;
use crate::world::{SimConfig, SimError, Snapshot, World};
use crate::{FormId, FormStore, RefTable};
use rand_chacha::ChaCha8Rng;

const PLAYER: FormId = FormId::PLAYER_REF;
const A: FormId = FormId(0x300);
const B: FormId = FormId(0x301);
const NO_INVENTORY: FormId = FormId(0x302);
const NPC_BASE: FormId = FormId(0x200);
const INPUT: FormId = FormId(0x100);
const OTHER: FormId = FormId(0x101);
const GEAR: FormId = FormId(0x102);
const CHOICE: FormId = FormId(0x110);
const CYCLE: FormId = FormId(0x111);
const UNKNOWN: FormId = FormId(0xFFFF);

struct Fixture { forms: FormStore, refs: RefTable }

fn item(kind: ItemKind) -> Item {
    Item {
        kind, value: Some(1), money: None, weight: Some(1.0), slots: None,
        projectile: None, health: Some(100), non_playable: false, cant_drop: false,
        armor_class: ArmorClass::Light, power_armor: false, armor_dr: None, armor_dt: None,
        weapon_damage: None, weight_mods: [0.0; 3], drawable: false, not_for_npcs: false,
        thrown: false, quest_item: false, icon: None, placed_count: None, teaches: None,
        cant_be_taken: false, modulates_voice: false, weapon_traits: None,
    }
}

fn fixture() -> Fixture {
    let mut forms = FormStore::default();
    let list = |entries: &[(FormId, u16)]| BaseData::LeveledItem(LeveledList {
        chance_none: 0, all_levels: true, each: false, use_all: false, global: None,
        entries: entries.iter().map(|&(item, count)| LeveledEntry { level: 1, item, count, condition: None }).collect(),
    });
    for (id, sig, data) in [
        (INPUT, *b"MISC", BaseData::Item(item(ItemKind::Misc))),
        (OTHER, *b"MISC", BaseData::Item(item(ItemKind::Misc))),
        (GEAR, *b"WEAP", BaseData::Item(item(ItemKind::Weapon))),
        (CHOICE, *b"LVLI", list(&[(GEAR, 1), (GEAR, 2)])),
        (CYCLE, *b"LVLI", list(&[(CYCLE, 1)])),
        (FormId::PLAYER_BASE, *b"NPC_", BaseData::Actor(ActorBase { entries: vec![], attributes: Some([5; 7]) })),
        (NPC_BASE, *b"NPC_", BaseData::Actor(ActorBase { entries: vec![Entry::new(INPUT, 4), Entry::new(CHOICE, 1)], attributes: Some([5; 7]) })),
    ] {
        forms.sigs.insert(id, sig);
        forms.bases.insert(id, BaseForm { id, sig, edid: None, name: None, script: None, prompt: None, voice_filter: false, data });
    }
    let mut refs = RefTable::default();
    for id in [A, B, NO_INVENTORY] {
        refs.refs.insert(id, PlacedRef {
            id, sig: *b"ACHR", edid: None, base: if id == NO_INVENTORY { INPUT } else { NPC_BASE },
            cell: None, stored_in: None, plugin: 0, flags: 0, in_persistent_group: false,
            pos: [0.0; 3], rot: [0.0; 3], scale: None, count: None, ammo: None,
            health: None, owner: None, lock: None, teleport: None, enable_parent: None,
            linked: None, patrol_idle: None, patrol_topic: None, merchant_container: None,
            marker_reputation: None, open_by_default: false, primitive: None,
            activate_parents: vec![], parent_activate_only: false, prompt: None,
            radiation: None, radius: None, encounter_zone: None, level_modifier: None,
        });
    }
    Fixture { forms, refs }
}

fn world(f: &Fixture) -> World<'_> { World::new(&f.forms, &f.refs, SimConfig::default()) }

fn unchanged(w: &World<'_>, before: &Snapshot, rng: &ChaCha8Rng, presence: u64) {
    assert_eq!(&w.snapshot(), before, "all saved state, including absent refs/death items, must remain exact");
    assert_eq!(&w.rng, rng, "the full RNG state must remain exact");
    assert_eq!(w.presence_serial(), presence);
}

#[test]
fn success_matches_real_ordinary_transfers_and_returns_aggregated_actual_totals() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 6).unwrap();
    w.add_item(PLAYER, OTHER, 3).unwrap();
    w.get(B).unwrap();
    let mut ordinary = World::restore(&f.forms, &f.refs, &w.snapshot()).unwrap();
    ordinary.transfer(PLAYER, B, INPUT, 4).unwrap();
    ordinary.transfer(PLAYER, B, OTHER, 2).unwrap();

    assert_eq!(w.transfer_inventory_exact(PLAYER, B, &[(OTHER, 2), (INPUT, 1), (INPUT, 3)]).unwrap(),
        [(INPUT, 4), (OTHER, 2)]);
    assert_eq!(w.snapshot(), ordinary.snapshot());
    assert_eq!(w.rng, ordinary.rng);
}

#[test]
fn second_missing_item_rolls_back_both_real_inventories() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 3).unwrap();
    w.get(B).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact(PLAYER, B, &[(INPUT, 2), (OTHER, 1)]),
        Err(InventoryTransferError::Missing { item: OTHER, have: 0, need: 1 }));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn destination_total_overflow_after_an_earlier_item_preserves_all_condition_groups() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    w.add_item(PLAYER, OTHER, 2).unwrap();
    w.add_item_extra(B, OTHER, i32::MAX - 1, Extra { condition: Some(0.5), ..Extra::default() }).unwrap();
    w.add_item_extra(B, OTHER, 1, Extra { condition: Some(0.8), ..Extra::default() }).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact(PLAYER, B, &[(INPUT, 1), (OTHER, 1)]),
        Err(InventoryTransferError::CountOverflow(OTHER)));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn failure_restores_two_unmaterialized_holders_their_death_items_and_rng() {
    let f = fixture();
    let mut w = world(&f);
    w.add_death_item(A, CHOICE).unwrap();
    w.add_death_item(B, CHOICE).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    let mut control = World::restore(&f.forms, &f.refs, &before).unwrap();
    control.get(B).unwrap(); control.get(A).unwrap();
    assert_ne!(control.rng, rng, "the fixture really consumes random base/death rolls");
    assert!(control.owed_death_items().is_empty());

    assert!(matches!(w.transfer_inventory_exact(A, B, &[(INPUT, 1), (OTHER, 1)]),
        Err(InventoryTransferError::Missing { item: OTHER, .. })));
    assert!(w.peek(A).is_none() && w.peek(B).is_none());
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn source_death_roll_error_restores_already_materialized_candidate_destination() {
    let f = fixture();
    let mut w = world(&f);
    w.add_death_item(A, CYCLE).unwrap(); w.add_death_item(B, CHOICE).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert!(w.transfer_inventory_exact(A, B, &[(INPUT, 1)]).is_err());
    assert!(w.peek(A).is_none() && w.peek(B).is_none());
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn duplicate_total_missing_is_checked_together_and_never_partially_moves() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 3).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact(PLAYER, B, &[(INPUT, 2), (INPUT, 2)]),
        Err(InventoryTransferError::Missing { item: INPUT, have: 3, need: 4 }));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn overflowing_duplicate_request_is_refused_before_materializing_either_holder() {
    let f = fixture();
    let mut w = world(&f);
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact(A, B, &[(INPUT, i32::MAX), (INPUT, 1)]),
        Err(InventoryTransferError::CountOverflow(INPUT)));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn actual_extra_groups_move_unequipped_and_unmoved_equipment_is_untouched() {
    let f = fixture();
    let mut w = world(&f);
    let owned = Extra { condition: Some(0.42), owner: Some(FormId(0x888)), equipped: true, mods: 5 };
    let other = Extra { condition: Some(0.9), owner: Some(FormId(0x889)), equipped: true, mods: 2 };
    w.add_item_extra(PLAYER, GEAR, 2, owned).unwrap();
    w.add_item_extra(PLAYER, OTHER, 1, other).unwrap();
    w.get(PLAYER).unwrap().scale = Some(1.25);
    let destination_before = w.inventory_mut(B).unwrap().count(GEAR);
    assert_eq!(w.transfer_inventory_exact(PLAYER, B, &[(GEAR, 1)]).unwrap(), [(GEAR, 1)]);
    let source = w.peek(PLAYER).unwrap().inventory.as_ref().unwrap();
    assert_eq!(source.groups().filter(|g| g.0 == GEAR).collect::<Vec<_>>(), [(GEAR, owned, 1)]);
    assert_eq!(source.groups().filter(|g| g.0 == OTHER).collect::<Vec<_>>(), [(OTHER, other, 1)]);
    let destination = w.peek(B).unwrap().inventory.as_ref().unwrap();
    assert!(destination.groups().any(|g| g == (GEAR, Extra { equipped: false, ..owned }, 1)));
    assert_eq!(destination.count(GEAR), destination_before + 1);
    assert_eq!(w.peek(PLAYER).unwrap().scale, Some(1.25));
}

#[test]
fn same_holder_rejected_even_for_empty_batch_without_any_materialization() {
    let f = fixture();
    let mut w = world(&f);
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    for items in [&[][..], &[(INPUT, 1)][..]] {
        assert_eq!(w.transfer_inventory_exact(A, A, items), Err(InventoryTransferError::SameHolder(A)));
        unchanged(&w, &before, &rng, presence);
    }
}

#[test]
fn bad_counts_unknown_and_non_item_rows_leave_everything_unchanged() {
    let f = fixture();
    let mut w = world(&f);
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    for count in [0, -1, i32::MIN] {
        assert_eq!(w.transfer_inventory_exact(A, B, &[(INPUT, 1), (OTHER, count)]),
            Err(InventoryTransferError::Sim(SimError::BadCount(count))));
        unchanged(&w, &before, &rng, presence);
    }
    for (item, expected) in [(UNKNOWN, SimError::UnknownForm(UNKNOWN)), (NPC_BASE, SimError::NotAnItem(NPC_BASE)), (CHOICE, SimError::NotAnItem(CHOICE))] {
        assert_eq!(w.transfer_inventory_exact(A, B, &[(INPUT, 1), (item, 1)]), Err(InventoryTransferError::Sim(expected)));
        unchanged(&w, &before, &rng, presence);
    }
}

#[test]
fn unknown_source_after_destination_roll_and_no_inventory_destination_are_atomic() {
    let f = fixture();
    let mut w = world(&f);
    w.add_death_item(B, CHOICE).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact(UNKNOWN, B, &[(INPUT, 1)]),
        Err(InventoryTransferError::Sim(SimError::UnknownRef(UNKNOWN))));
    unchanged(&w, &before, &rng, presence);
    assert_eq!(w.transfer_inventory_exact(A, NO_INVENTORY, &[(INPUT, 1)]),
        Err(InventoryTransferError::Sim(SimError::NoInventory(NO_INVENTORY))));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn failed_first_player_materialization_restores_its_presence_serial_increment() {
    let f = fixture();
    let mut w = world(&f);
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    let mut control = World::restore(&f.forms, &f.refs, &before).unwrap();
    let previous = control.presence_serial(); control.get(PLAYER).unwrap();
    assert_ne!(previous, control.presence_serial());
    assert!(w.transfer_inventory_exact(PLAYER, B, &[(INPUT, 1)]).is_err());
    assert!(w.peek(PLAYER).is_none() && w.peek(B).is_none());
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn successful_real_transfer_survives_json_snapshot_roundtrip_and_future_random_rolls() {
    let f = fixture();
    let mut w = world(&f);
    w.add_death_item(A, CHOICE).unwrap(); w.add_death_item(B, CHOICE).unwrap();
    let before_total = 8;
    assert_eq!(w.transfer_inventory_exact(A, B, &[(INPUT, 3)]).unwrap(), [(INPUT, 3)]);
    assert_eq!(w.item_count(A, INPUT).unwrap() + w.item_count(B, INPUT).unwrap(), before_total);
    let saved = w.snapshot();
    let decoded: Snapshot = serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut restored = World::restore(&f.forms, &f.refs, &decoded).unwrap();
    assert_eq!(restored.snapshot(), saved);
    assert_eq!(restored.rng, w.rng);
    assert_eq!(restored.add_item(A, CHOICE, 3).unwrap(), w.add_item(A, CHOICE, 3).unwrap());
    assert_eq!(restored.snapshot(), w.snapshot());
}

#[test]
fn selected_condition_mod_rows_move_only_their_real_groups_and_aggregate_totals() {
    let f = fixture();
    let mut w = world(&f);
    let first = Extra { condition: Some(0.4), owner: Some(FormId(0x888)), equipped: true, mods: 1 };
    let second = Extra { condition: Some(0.8), owner: Some(FormId(0x889)), equipped: false, mods: 4 };
    let untouched = Extra { condition: Some(0.4), owner: Some(FormId(0x890)), equipped: true, mods: 2 };
    w.add_item_extra(PLAYER, GEAR, 2, first).unwrap();
    w.add_item_extra(PLAYER, GEAR, 3, second).unwrap();
    w.add_item_extra(PLAYER, GEAR, 4, untouched).unwrap();
    let before = w.snapshot();
    let mut ordinary = World::restore(&f.forms, &f.refs, &before).unwrap();
    ordinary.transfer_at(PLAYER, B, GEAR, 2, Pick::row(Some(0.8), 4)).unwrap();
    ordinary.transfer_at(PLAYER, B, GEAR, 1, Pick::row(Some(0.4), 1)).unwrap();

    assert_eq!(w.transfer_inventory_exact_at(PLAYER, B, &[
        (GEAR, 2, Pick::row(Some(0.8), 4)), (GEAR, 1, Pick::row(Some(0.4), 1)),
    ]).unwrap(), [(GEAR, 3)]);
    assert_eq!(w.snapshot(), ordinary.snapshot());
    assert_eq!(w.rng, ordinary.rng);
    let source = w.peek(PLAYER).unwrap().inventory.as_ref().unwrap();
    assert!(source.groups().any(|group| group == (GEAR, untouched, 4)));
    assert!(source.groups().any(|group| group == (GEAR, first, 1)));
    assert!(source.groups().any(|group| group == (GEAR, second, 1)));
    let destination = w.peek(B).unwrap().inventory.as_ref().unwrap();
    assert!(destination.groups().any(|group| group == (GEAR, Extra { equipped: false, ..first }, 1)));
    assert!(destination.groups().any(|group| group == (GEAR, second, 2)));
}

#[test]
fn overlapping_selected_rows_cannot_reuse_units_and_short_later_pick_rolls_back() {
    let f = fixture();
    let selected = Pick::row(Some(0.5), 3);
    // Cover identical picks and ANY overlapping a narrower following pick. Picks
    // are requests against the remaining candidate, never independent stock reads.
    for first_pick in [selected, Pick::ANY] {
        let mut w = world(&f);
        w.add_item_extra(PLAYER, GEAR, 2, Extra { condition: Some(0.5), owner: Some(FormId(0x888)), equipped: true, mods: 3 }).unwrap();
        let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
        assert_eq!(w.transfer_inventory_exact_at(PLAYER, B, &[(GEAR, 2, first_pick), (GEAR, 1, selected)]),
            Err(InventoryTransferError::Missing { item: GEAR, have: 0, need: 1 }));
        assert!(w.peek(B).is_none(), "even the initially rolled destination must be removed");
        unchanged(&w, &before, &rng, presence);
    }
}

#[test]
fn overflowing_selected_row_rolls_back_an_earlier_successful_candidate_row() {
    let f = fixture();
    let mut w = world(&f);
    let selected = Extra { condition: Some(0.8), owner: Some(FormId(0x888)), equipped: true, mods: 4 };
    w.add_item(PLAYER, INPUT, 2).unwrap();
    w.add_item_extra(PLAYER, OTHER, 1, selected).unwrap();
    w.add_item_extra(B, OTHER, i32::MAX, Extra { condition: Some(0.9), ..Extra::default() }).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact_at(PLAYER, B, &[
        (INPUT, 1, Pick::ANY), (OTHER, 1, Pick::row(Some(0.8), 4)),
    ]), Err(InventoryTransferError::CountOverflow(OTHER)));
    unchanged(&w, &before, &rng, presence);
}

#[test]
fn nan_pick_matches_no_real_condition_and_refuses_the_whole_batch() {
    let f = fixture();
    let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, Extra { condition: Some(0.5), mods: 1, ..Extra::default() }).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let presence = w.presence_serial();
    assert_eq!(w.transfer_inventory_exact_at(PLAYER, B, &[(INPUT, 1, Pick::ANY), (GEAR, 1, Pick::row(Some(f32::NAN), 1))]),
        Err(InventoryTransferError::Missing { item: GEAR, have: 0, need: 1 }));
    unchanged(&w, &before, &rng, presence);
}
