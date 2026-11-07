//! Actual World escrow APIs over synthetic records; no game files or cloned algorithm.

use super::EscrowError;
use crate::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Container, Entry, Item, ItemKind, LeveledEntry, LeveledList};
use crate::inventory::Extra;
use crate::refs::PlacedRef;
use crate::world::{InventoryExchangeError, InventoryTransferError, SimConfig, SimError, Snapshot, World};
use crate::{FormId, FormStore, RefTable};
use rand_chacha::ChaCha8Rng;

const PLAYER: FormId = FormId::PLAYER_REF;
const SOURCE: FormId = FormId(0x310);
const ANCHOR: FormId = FormId(0x311);
const NPC: FormId = FormId(0x210);
const BOX: FormId = FormId(0x220);
const RESPAWNS: FormId = FormId(0x221);
const SCRIPTED: FormId = FormId(0x222);
const STOCKED: FormId = FormId(0x223);
const INPUT: FormId = FormId(0x110);
const OTHER: FormId = FormId(0x111);
const OUTPUT: FormId = FormId(0x112);
const GEAR: FormId = FormId(0x113);
const LIST: FormId = FormId(0x114);
const UNKNOWN: FormId = FormId(0xEEFF);
const TOKEN: u64 = 41;

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
    for (id, sig, data, script) in [
        (INPUT, *b"MISC", BaseData::Item(item(ItemKind::Misc)), None),
        (OTHER, *b"MISC", BaseData::Item(item(ItemKind::Misc)), None),
        (OUTPUT, *b"MISC", BaseData::Item(item(ItemKind::Misc)), None),
        (GEAR, *b"WEAP", BaseData::Item(item(ItemKind::Weapon)), None),
        (BOX, *b"CONT", BaseData::Container(Container { respawns: false, entries: vec![] }), None),
        (RESPAWNS, *b"CONT", BaseData::Container(Container { respawns: true, entries: vec![] }), None),
        (SCRIPTED, *b"CONT", BaseData::Container(Container { respawns: false, entries: vec![] }), Some(FormId(0x999))),
        (STOCKED, *b"CONT", BaseData::Container(Container { respawns: false, entries: vec![Entry::new(INPUT, 2)] }), None),
        (FormId::PLAYER_BASE, *b"NPC_", BaseData::Actor(ActorBase { entries: vec![], attributes: Some([5; 7]) }), None),
        (NPC, *b"NPC_", BaseData::Actor(ActorBase { entries: vec![Entry::new(INPUT, 4), Entry::new(LIST, 1)], attributes: Some([5; 7]) }), None),
        (LIST, *b"LVLI", BaseData::LeveledItem(LeveledList {
            chance_none: 0, all_levels: true, each: false, use_all: false, global: None,
            entries: vec![LeveledEntry { level: 1, item: GEAR, count: 1, condition: None }, LeveledEntry { level: 1, item: GEAR, count: 2, condition: None }],
        }), None),
    ] {
        forms.sigs.insert(id, sig);
        forms.bases.insert(id, BaseForm { id, sig, edid: None, name: None, script, prompt: None, voice_filter: false, data });
    }
    let mut refs = RefTable::default();
    for (id, base) in [(SOURCE, NPC), (ANCHOR, BOX)] {
        refs.refs.insert(id, PlacedRef {
            id, sig: if id == SOURCE { *b"ACHR" } else { *b"REFR" }, edid: None, base,
            cell: Some(FormId(0x500)), stored_in: None, plugin: 0, flags: 0, in_persistent_group: false,
            pos: [100.0, 200.0, 300.0], rot: [0.0, 0.0, 0.25], scale: None, count: None,
            ammo: None, health: None, owner: None, lock: None, teleport: None,
            enable_parent: None, linked: None, patrol_idle: None, patrol_topic: None,
            merchant_container: None, marker_reputation: None, open_by_default: false,
            primitive: None, activate_parents: vec![], parent_activate_only: false,
            prompt: None, radiation: None, radius: None, encounter_zone: None, level_modifier: None,
        });
    }
    Fixture { forms, refs }
}

fn world(f: &Fixture) -> World<'_> { World::new(&f.forms, &f.refs, SimConfig::default()) }

fn stock(w: &mut World<'_>) {
    w.add_item(PLAYER, INPUT, 4).unwrap();
    w.add_item(PLAYER, OTHER, 2).unwrap();
}

fn count(w: &World<'_>, holder: FormId, item: FormId) -> i32 {
    w.peek(holder).unwrap().inventory.as_ref().unwrap().count(item)
}

fn unchanged(w: &World<'_>, snapshot: &Snapshot, rng: &ChaCha8Rng, presence: u64) {
    assert_eq!(&w.snapshot(), snapshot);
    assert_eq!(&w.rng, rng);
    assert_eq!(w.presence_serial(), presence);
}

#[test]
fn create_funds_one_actual_hidden_owned_inventory_at_the_anchor() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    assert!(w.peek(ANCHOR).is_none());
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1), (INPUT, 2), (OTHER, 1)], TOKEN).unwrap();
    assert_eq!(escrow, FormId::FIRST_DYNAMIC);
    let state = w.peek(escrow).unwrap();
    assert_eq!((state.base, state.cell, state.pos, state.rot), (BOX, Some(FormId(0x500)), [100.0, 200.0, 300.0], [0.0, 0.0, 0.25]));
    assert!(!state.enabled() && !state.deleted());
    assert_eq!(state.extra.owner, Some(FormId::PLAYER_BASE));
    assert_eq!(w.inventory_reservation(escrow), Some(TOKEN));
    assert_eq!((count(&w, PLAYER, INPUT), count(&w, escrow, INPUT)), (1, 3));
    assert_eq!((count(&w, PLAYER, OTHER), count(&w, escrow, OTHER)), (1, 1));
    assert!(w.peek(ANCHOR).is_none(), "reading the anchor must not create a third mutable holder");
}

#[test]
fn missing_later_input_rolls_back_source_rng_death_roll_and_allocated_id() {
    let f = fixture(); let mut w = world(&f);
    w.add_death_item(SOURCE, LIST).unwrap();
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert!(matches!(w.create_production_escrow(SOURCE, BOX, ANCHOR, &[(INPUT, 2), (OTHER, 1)], TOKEN),
        Err(EscrowError::Transfer(InventoryTransferError::Missing { item: OTHER, .. }))));
    assert!(w.peek(SOURCE).is_none());
    assert!(w.created().next().is_none());
    unchanged(&w, &snapshot, &rng, serial);
    let escrow = w.create_production_escrow(SOURCE, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    assert_eq!(escrow, FormId::FIRST_DYNAMIC, "failed reservation must not consume an id");
}

#[test]
fn invalid_storage_types_scripts_respawn_and_initial_stock_never_allocate() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    for base in [INPUT, NPC, RESPAWNS, SCRIPTED, STOCKED] {
        assert_eq!(w.create_production_escrow(PLAYER, base, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::InvalidStorageBase(base)));
        unchanged(&w, &snapshot, &rng, serial);
    }
    assert_eq!(w.create_production_escrow(PLAYER, UNKNOWN, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::Sim(SimError::UnknownForm(UNKNOWN))));
    unchanged(&w, &snapshot, &rng, serial);
}

#[test]
fn a_session_base_stock_edit_cannot_smuggle_initial_goods_into_storage() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    w.base_edits.insert(BOX, vec![Entry::new(OUTPUT, 9)]);
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::InvalidStorageBase(BOX)));
    unchanged(&w, &snapshot, &rng, serial);
    assert_eq!(w.base_edits.get(&BOX).unwrap()[0].count, 9);
}

#[test]
fn unknown_anchor_and_source_bad_quantity_and_request_overflow_do_not_leak_ids() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.create_production_escrow(PLAYER, BOX, UNKNOWN, &[(INPUT, 1)], TOKEN), Err(EscrowError::Sim(SimError::UnknownRef(UNKNOWN))));
    unchanged(&w, &snapshot, &rng, serial);
    assert_eq!(w.create_production_escrow(UNKNOWN, BOX, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::Transfer(InventoryTransferError::Sim(SimError::UnknownRef(UNKNOWN)))));
    unchanged(&w, &snapshot, &rng, serial);
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 0)], TOKEN), Err(EscrowError::Transfer(InventoryTransferError::Sim(SimError::BadCount(0)))));
    unchanged(&w, &snapshot, &rng, serial);
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, i32::MAX), (INPUT, 1)], TOKEN), Err(EscrowError::Transfer(InventoryTransferError::CountOverflow(INPUT))));
    unchanged(&w, &snapshot, &rng, serial);
}

#[test]
fn dynamic_id_exhaustion_or_collision_refuses_creation_without_overwriting_state() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    for next in [0xFEFF_FFFF, u32::MAX] {
        w.next_dynamic = next;
        let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
        assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::DynamicIdsExhausted));
        unchanged(&w, &snapshot, &rng, serial);
    }
    w.next_dynamic = FormId::FIRST_DYNAMIC.0;
    let id = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN).unwrap();
    w.next_dynamic = id.0;
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN + 1), Err(EscrowError::DynamicIdCollision(id)));
    unchanged(&w, &snapshot, &rng, serial);
}

#[test]
fn live_token_reuse_zero_and_wrong_token_leave_inventory_and_reservation_exact() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], 0), Err(EscrowError::InvalidToken));
    assert_eq!(w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN), Err(EscrowError::TokenInUse(TOKEN)));
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN + 1, &[(INPUT, 1)], &[(OUTPUT, 1)]), Err(EscrowError::WrongToken(escrow)));
    assert_eq!(w.release_production_escrow(escrow, TOKEN + 1, PLAYER), Err(EscrowError::WrongToken(escrow)));
    assert_eq!(w.release_production_escrow(escrow, 0, PLAYER), Err(EscrowError::InvalidToken));
    unchanged(&w, &snapshot, &rng, serial);
}

#[test]
fn ordinary_inventory_mutations_and_delete_are_blocked_while_reserved() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    let snapshot = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.add_item(escrow, OUTPUT, 1), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(w.remove_item(escrow, INPUT, 1), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(w.transfer(escrow, PLAYER, INPUT, 1), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(w.transfer(PLAYER, escrow, INPUT, 1), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(w.inventory_mut(escrow).unwrap_err(), SimError::ReservedInventory(escrow));
    assert_eq!(w.set_deleted(escrow, true), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &snapshot, &rng, serial);
}

#[test]
fn reset_inventory_cannot_erase_funded_inputs_or_pending_death_metadata_after_late_source_edits() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    // A session edit arriving after admission must not make ResetInventory a way
    // around the seal. These maps are test setup, not authorized escrow mutations.
    w.created_inventory_sources.insert(escrow, NPC);
    w.death_items.insert(escrow, LIST);
    w.base_edits.insert(BOX, vec![Entry::new(LIST, 1)]);
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.reset_inventory(escrow), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(count(&w, escrow, INPUT), 2);
    assert_eq!(w.owed_death_items().get(&escrow), Some(&LIST));
    assert_eq!(w.created_inventory_sources.get(&escrow), Some(&NPC));
}

#[test]
fn give_items_of_cannot_replace_funded_stock_or_change_inventory_source_mappings() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.created_inventory_sources.insert(escrow, STOCKED);
    w.set_inventory_source(escrow, Some(NPC));
    let sources = w.inventory_sources.clone();
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    for replacement in [NPC, STOCKED, UNKNOWN] {
        assert_eq!(w.give_items_of(escrow, replacement), Err(SimError::ReservedInventory(escrow)));
        unchanged(&w, &before, &rng, serial);
        assert_eq!(w.inventory_sources, sources);
    }
    assert_eq!(count(&w, escrow, INPUT), 2);
}

#[test]
fn outfit_cannot_take_or_equip_reserved_condition_groups_even_with_an_actor_source() {
    let f = fixture(); let mut w = world(&f);
    let worn = Extra { condition: Some(0.6), mods: 3, owner: Some(FormId(0x800)), equipped: false };
    w.add_item_extra(PLAYER, GEAR, 2, worn).unwrap();
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(GEAR, 2)], TOKEN).unwrap();
    w.created_inventory_sources.insert(escrow, NPC);
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.outfit(escrow), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert!(!w.peek(escrow).unwrap().outfitted);
    assert_eq!(w.peek(escrow).unwrap().inventory.as_ref().unwrap().groups().collect::<Vec<_>>(), [(GEAR, worn, 2)]);
}

#[test]
fn death_item_rejection_precedes_leveled_rng_and_owed_item_changes() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.death_items.insert(escrow, OTHER);
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.add_death_item(escrow, LIST), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.owed_death_items().get(&escrow), Some(&OTHER));
    assert_eq!(count(&w, escrow, GEAR), 0);
}

#[test]
fn automatic_resets_and_restocks_skip_a_reserved_holder_after_late_base_changes() {
    let mut f = fixture();
    // Normally escrow is dynamic and nonrespawning. Deliberately arrange both
    // automatic indexes plus a late base change to test defense at each writer.
    f.refs.by_cell.insert(FormId(0x500), vec![FormId::FIRST_DYNAMIC]);
    f.refs.merchant_containers.insert(FormId::FIRST_DYNAMIC, vec![SOURCE]);
    let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.get(escrow).unwrap().base = RESPAWNS;
    w.base_edits.insert(RESPAWNS, vec![Entry::new(LIST, 1)]);
    w.ref_levels.insert(escrow, crate::zones::LevelAt::plain(8));
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert!(w.reset_containers(FormId(0x500)).is_empty());
    unchanged(&w, &before, &rng, serial);
    w.restock_merchants(0.0, 7.0).unwrap();
    unchanged(&w, &before, &rng, serial);
    assert_eq!(count(&w, escrow, INPUT), 2);
}

#[test]
fn rewound_dynamic_cursor_cannot_overwrite_escrow_or_take_drop_source_groups() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let equipped = Extra { condition: Some(0.7), mods: 2, owner: Some(FormId(0x800)), equipped: true };
    w.add_item_extra(PLAYER, GEAR, 1, equipped).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, Extra { condition: Some(0.4), ..Extra::default() }).unwrap();
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.next_dynamic = escrow.0;
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.place(NPC, Some(FormId(0x500)), [0.0; 3], [0.0; 3], 1), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.drop_item(PLAYER, GEAR, 2), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.drop_item_fresh(PLAYER, GEAR, 2), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.drop_equipped(PLAYER, GEAR), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.player_drop_item(GEAR, 2), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
}

#[test]
fn a_reserved_later_dynamic_id_is_rejected_before_a_multigroup_drop() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let earlier = w.place(BOX, Some(FormId(0x500)), [0.0; 3], [0.0; 3], 1).unwrap();
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 1)], TOKEN).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, Extra { condition: Some(0.4), ..Extra::default() }).unwrap();
    w.add_item_extra(PLAYER, GEAR, 1, Extra { condition: Some(0.8), ..Extra::default() }).unwrap();
    w.next_dynamic = earlier.0;
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.drop_item(PLAYER, GEAR, 2), Err(SimError::ReservedInventory(escrow)));
    unchanged(&w, &before, &rng, serial);
}

#[test]
fn ordinary_reset_source_rolls_outfit_and_drops_still_work_for_unreserved_holders() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    w.give_items_of(SOURCE, STOCKED).unwrap();
    assert_eq!(count(&w, SOURCE, INPUT), 2);
    w.remove_item(SOURCE, INPUT, 1).unwrap();
    assert_eq!(w.reset_inventory(SOURCE), Ok(true));
    assert_eq!(count(&w, SOURCE, INPUT), 2);
    assert_eq!(w.outfit(SOURCE), Ok(false), "a container inventory source does not outfit an actor");
    w.add_death_item(SOURCE, LIST).unwrap();
    assert!((1..=2).contains(&count(&w, SOURCE, GEAR)));
    let actor = w.place(NPC, Some(FormId(0x500)), [0.0; 3], [0.0; 3], 1).unwrap();
    assert_eq!(count(&w, actor, INPUT), 4);
    let dropped = w.drop_item(SOURCE, INPUT, 1).unwrap();
    assert_eq!(dropped.len(), 1);
    assert_eq!(w.peek(dropped[0]).unwrap().base, INPUT);
    assert_eq!(count(&w, SOURCE, INPUT), 1);
}

#[test]
fn completed_escrow_keeps_ordinary_write_guards_after_a_real_json_reload() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(OUTPUT, 1)]).unwrap();
    let snapshot: Snapshot = serde_json::from_str(&serde_json::to_string(&w.snapshot()).unwrap()).unwrap();
    let mut loaded = World::restore(&f.forms, &f.refs, &snapshot).unwrap();
    let before = loaded.snapshot(); let rng = loaded.rng.clone(); let serial = loaded.presence_serial();
    assert_eq!(loaded.reset_inventory(escrow), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(loaded.give_items_of(escrow, NPC), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(loaded.outfit(escrow), Err(SimError::ReservedInventory(escrow)));
    assert_eq!(loaded.add_death_item(escrow, LIST), Err(SimError::ReservedInventory(escrow)));
    unchanged(&loaded, &before, &rng, serial);
    assert_eq!(loaded.release_production_escrow(escrow, TOKEN, PLAYER).unwrap(), [(OUTPUT, 1)]);
}

#[test]
fn completion_consumes_inputs_once_and_collection_moves_only_actual_outputs() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2), (OTHER, 1)], TOKEN).unwrap();
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2), (OTHER, 1)], &[(OUTPUT, 3)]).unwrap(), [(OUTPUT, 3)]);
    assert_eq!((count(&w, escrow, INPUT), count(&w, escrow, OTHER), count(&w, escrow, OUTPUT)), (0, 0, 3));
    assert_eq!(w.inventory_reservation(escrow), Some(TOKEN));
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert!(matches!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2), (OTHER, 1)], &[(OUTPUT, 3)]), Err(EscrowError::Exchange(InventoryExchangeError::Missing { .. }))));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.release_production_escrow(escrow, TOKEN, PLAYER).unwrap(), [(OUTPUT, 3)]);
    assert_eq!((count(&w, PLAYER, INPUT), count(&w, PLAYER, OTHER), count(&w, PLAYER, OUTPUT)), (2, 1, 3));
    assert_eq!(w.inventory_reservation(escrow), None);
    assert!(w.peek(escrow).unwrap().deleted());
    assert_eq!(w.release_production_escrow(escrow, TOKEN, PLAYER), Err(EscrowError::NotReserved(escrow)));
}

#[test]
fn completion_failure_keeps_real_inputs_and_the_token_for_a_later_retry() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(UNKNOWN, 1)]), Err(EscrowError::Exchange(InventoryExchangeError::Sim(SimError::UnknownForm(UNKNOWN)))));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(OUTPUT, 1)]).unwrap(), [(OUTPUT, 1)]);
}

#[test]
fn completion_destination_quantity_overflow_is_atomic_with_the_reservation() {
    let f = fixture(); let mut w = world(&f);
    w.add_item(PLAYER, INPUT, 2).unwrap(); w.add_item(PLAYER, OUTPUT, i32::MAX).unwrap();
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2), (OUTPUT, i32::MAX)], TOKEN).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(OUTPUT, 1)]), Err(EscrowError::Exchange(InventoryExchangeError::CountOverflow(OUTPUT))));
    unchanged(&w, &before, &rng, serial);
}

#[test]
fn collection_capacity_failure_and_same_holder_keep_goods_and_reservation() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(OUTPUT, 1)]).unwrap();
    w.add_item(PLAYER, OUTPUT, i32::MAX).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.release_production_escrow(escrow, TOKEN, PLAYER), Err(EscrowError::Transfer(InventoryTransferError::CountOverflow(OUTPUT))));
    unchanged(&w, &before, &rng, serial);
    assert_eq!(w.release_production_escrow(escrow, TOKEN, escrow), Err(EscrowError::Transfer(InventoryTransferError::SameHolder(escrow))));
    unchanged(&w, &before, &rng, serial);
    w.remove_item(PLAYER, OUTPUT, 1).unwrap();
    w.release_production_escrow(escrow, TOKEN, PLAYER).unwrap();
    assert_eq!(count(&w, PLAYER, OUTPUT), i32::MAX);
}

#[test]
fn cancellation_refunds_actual_condition_mod_owner_groups_unequipped() {
    let f = fixture(); let mut w = world(&f);
    let extra = Extra { condition: Some(0.43), mods: 5, owner: Some(FormId(0x800)), equipped: true };
    w.add_item_extra(PLAYER, GEAR, 2, extra).unwrap();
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(GEAR, 1)], TOKEN).unwrap();
    let moved = Extra { equipped: false, ..extra };
    assert!(w.peek(escrow).unwrap().inventory.as_ref().unwrap().groups().any(|g| g == (GEAR, moved, 1)));
    assert_eq!(w.release_production_escrow(escrow, TOKEN, PLAYER).unwrap(), [(GEAR, 1)]);
    let groups: Vec<_> = w.peek(PLAYER).unwrap().inventory.as_ref().unwrap().groups().collect();
    assert!(groups.contains(&(GEAR, extra, 1)) && groups.contains(&(GEAR, moved, 1)));
}

#[test]
fn missing_release_target_and_storage_identity_tampering_do_not_drop_a_reservation() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    let before = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.release_production_escrow(escrow, TOKEN, UNKNOWN), Err(EscrowError::Transfer(InventoryTransferError::Sim(SimError::UnknownRef(UNKNOWN)))));
    unchanged(&w, &before, &rng, serial);
    w.get(escrow).unwrap().extra.owner = Some(FormId(0x777));
    let before = w.snapshot();
    assert_eq!(w.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 1)], &[(OUTPUT, 1)]), Err(EscrowError::InvalidEscrow(escrow)));
    assert_eq!(w.release_production_escrow(escrow, TOKEN, PLAYER), Err(EscrowError::InvalidEscrow(escrow)));
    unchanged(&w, &before, &rng, serial);
}

#[test]
fn reserved_working_and_completed_inventories_survive_real_json_snapshot_load() {
    let f = fixture(); let mut w = world(&f); stock(&mut w);
    let escrow = w.create_production_escrow(PLAYER, BOX, ANCHOR, &[(INPUT, 2)], TOKEN).unwrap();
    let encoded = serde_json::to_string(&w.snapshot()).unwrap();
    let snapshot: Snapshot = serde_json::from_str(&encoded).unwrap();
    let mut loaded = World::restore(&f.forms, &f.refs, &snapshot).unwrap();
    assert_eq!(loaded.snapshot(), w.snapshot());
    assert_eq!(loaded.inventory_reservation(escrow), Some(TOKEN));
    loaded.exchange_production_escrow(escrow, TOKEN, &[(INPUT, 2)], &[(OUTPUT, 3)]).unwrap();
    let encoded = serde_json::to_string(&loaded.snapshot()).unwrap();
    let snapshot: Snapshot = serde_json::from_str(&encoded).unwrap();
    let mut ready = World::restore(&f.forms, &f.refs, &snapshot).unwrap();
    assert_eq!(ready.inventory_reservation(escrow), Some(TOKEN));
    ready.release_production_escrow(escrow, TOKEN, PLAYER).unwrap();
    assert_eq!(count(&ready, PLAYER, OUTPUT), 3);
    let saved = ready.snapshot();
    assert_eq!(saved.deleted_created.get(&escrow), Some(&BOX));
    let mut closed = World::restore(&f.forms, &f.refs, &saved).unwrap();
    assert_eq!(closed.release_production_escrow(escrow, TOKEN, PLAYER), Err(EscrowError::NotReserved(escrow)));
    assert_eq!(count(&closed, PLAYER, OUTPUT), 3);
}
