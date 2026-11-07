//! [design] Actual World clock/save/reservation tests over synthetic public data.

use super::*;
use crate::forms::{BaseForm, Container, Entry, LeveledEntry, LeveledList};

const HOUR: FormId = FormId(0x38);
const DAYS: FormId = FormId(0x39);
const SCALE: FormId = FormId(0x3A);
const BOX: FormId = FormId(0x100);
const CYCLE: FormId = FormId(0x101);

fn forms() -> FormStore {
    let mut forms = FormStore::default();
    for (id, name, value) in [
        (FormId(0x35), clock::GAME_YEAR, 2281.0),
        (FormId(0x36), clock::GAME_MONTH, 9.0),
        (FormId(0x37), clock::GAME_DAY, 18.0),
        (HOUR, clock::GAME_HOUR, 12.0), (DAYS, clock::GAME_DAYS_PASSED, 5.0),
        (SCALE, clock::TIME_SCALE, 30.0),
    ] {
        forms.edids.insert(name.to_ascii_lowercase(), id);
        forms.globals.insert(id, value);
    }
    for (id, sig, data) in [
        (BOX, *b"CONT", BaseData::Container(Container { respawns: false, entries: vec![] })),
        (CYCLE, *b"LVLI", BaseData::LeveledItem(LeveledList {
            chance_none: 0, all_levels: true, each: false, use_all: false, global: None,
            entries: vec![LeveledEntry { level: 1, item: CYCLE, count: 1, condition: None }],
        })),
    ] {
        forms.sigs.insert(id, sig);
        forms.bases.insert(id, BaseForm { id, sig, edid: None, name: None, script: None, prompt: None, voice_filter: false, data });
    }
    forms
}

fn world<'a>(forms: &'a FormStore, refs: &'a RefTable) -> World<'a> {
    World::new(forms, refs, SimConfig::default())
}

#[test]
fn rust_snapshot_builder_default_has_the_same_valid_zero_as_missing_legacy_json() {
    let forms = forms(); let refs = RefTable::default();
    let snapshot = Snapshot { format: super::super::SNAPSHOT_FORMAT.into(), rng_word_pos: "0".into(), ..Default::default() };
    assert_eq!(snapshot.accepted_game_hours.as_deref(), Some("0"));
    assert_eq!(World::restore(&forms, &refs, &snapshot).unwrap().accepted_game_hours(), Some(0.0));
    let mut invalid = snapshot;
    invalid.accepted_game_hours = None;
    assert!(World::restore(&forms, &refs, &invalid).is_err(), "explicit null remains invalid; do not loosen the save gate");
}

#[test]
fn fractional_play_wait_and_to_hour_credit_the_same_total_once() {
    let forms = forms(); let refs = RefTable::default();
    let mut mixed = world(&forms, &refs);
    mixed.advance_real_seconds(60.0).unwrap(); // .5 game hours, not an extra .5 from the wrapper.
    mixed.advance_hours(0.25).unwrap();
    assert_eq!(mixed.advance_to_hour(13.0).unwrap(), 0.25);
    let mut waited = world(&forms, &refs);
    waited.advance_hours(1.0).unwrap();
    assert_eq!(mixed.accepted_game_hours(), Some(1.0));
    assert_eq!(mixed.accepted_game_hours(), waited.accepted_game_hours());
    assert_eq!(mixed.calendar().unwrap().hour, waited.calendar().unwrap().hour);
}

#[test]
fn direct_time_writes_including_same_value_never_grant_work() {
    let forms = forms(); let refs = RefTable::default(); let mut w = world(&forms, &refs);
    w.advance_hours(0.125).unwrap();
    for id in [HOUR, DAYS] {
        w.set_global(id, w.global(id).unwrap()).unwrap();
        w.set_global(id, 100.0).unwrap();
        w.set_global(id, 1.0).unwrap();
    }
    assert_eq!(w.accepted_game_hours(), Some(0.125));
    w.advance_hours(0.25).unwrap();
    assert_eq!(w.accepted_game_hours(), Some(0.375));
}

#[test]
fn zero_negative_nonfinite_and_target_hour_no_ops_do_not_credit() {
    let forms = forms(); let refs = RefTable::default(); let mut w = world(&forms, &refs);
    let calendar = w.calendar().unwrap();
    for hours in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(w.advance_hours(hours).unwrap(), calendar);
    }
    assert_eq!(w.advance_to_hour(12.0).unwrap(), 0.0);
    assert_eq!(w.advance_to_hour(f64::NAN).unwrap(), 0.0);
    assert_eq!(w.advance_to_hour(f64::INFINITY).unwrap(), 0.0);
    w.set_global(SCALE, 0.0).unwrap();
    w.advance_real_seconds(1000.0).unwrap();
    assert_eq!(w.accepted_game_hours(), Some(0.0));
}

#[test]
fn snapshot_keeps_counter_digits_and_restore_continues_the_same_stream() {
    let forms = forms(); let refs = RefTable::default(); let mut w = world(&forms, &refs);
    for _ in 0..137 { w.advance_hours(1.0 / 7200.0).unwrap(); }
    let saved = w.snapshot();
    let decoded: Snapshot = serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut restored = World::restore(&forms, &refs, &decoded).unwrap();
    assert_eq!(restored.accepted_game_hours(), w.accepted_game_hours());
    for _ in 0..71 {
        w.advance_real_seconds(1.0 / 60.0).unwrap();
        restored.advance_real_seconds(1.0 / 60.0).unwrap();
    }
    assert_eq!(restored.snapshot(), w.snapshot());
}

#[test]
fn old_snapshot_missing_counter_and_reservations_migrates_to_valid_zero() {
    let forms = forms(); let refs = RefTable::default(); let w = world(&forms, &refs);
    let mut json = serde_json::to_value(w.snapshot()).unwrap();
    json.as_object_mut().unwrap().remove("accepted_game_hours");
    json.as_object_mut().unwrap().remove("inventory_reservations");
    let old: Snapshot = serde_json::from_value(json).unwrap();
    let restored = World::restore(&forms, &refs, &old).unwrap();
    assert_eq!(restored.accepted_game_hours(), Some(0.0));
    assert!(restored.inventory_reservations.is_empty());
}

#[test]
fn malformed_or_invalid_saved_counter_is_rejected() {
    let forms = forms(); let refs = RefTable::default(); let w = world(&forms, &refs);
    for text in [None, Some(""), Some("bad"), Some("NaN"), Some("inf"), Some("-1"), Some("1e9999")] {
        let mut saved = w.snapshot(); saved.accepted_game_hours = text.map(str::to_owned);
        assert!(World::restore(&forms, &refs, &saved).is_err(), "must refuse {text:?}");
    }
}

#[test]
fn positive_precision_stall_and_overflow_invalidate_credit_without_recovery() {
    let forms = forms(); let refs = RefTable::default();
    let mut stalled = world(&forms, &refs);
    stalled.accepted_game_hours = Some(9_007_199_254_740_992.0);
    stalled.advance_hours(1.0).unwrap();
    assert_eq!(stalled.accepted_game_hours(), None);
    stalled.advance_hours(2.0).unwrap();
    assert_eq!(stalled.accepted_game_hours(), None);
    assert!(World::restore(&forms, &refs, &stalled.snapshot()).is_err());

    let mut overflow = world(&forms, &refs);
    overflow.accepted_game_hours = Some(f64::MAX);
    overflow.advance_hours(f64::MAX).unwrap();
    assert_eq!(overflow.accepted_game_hours(), None);
}

#[test]
fn calendar_store_restock_failure_does_not_grant_work() {
    let mut forms = forms(); let mut refs = RefTable::default();
    let BaseData::Container(container) = &mut forms.bases.get_mut(&BOX).unwrap().data else { unreachable!() };
    container.respawns = true;
    refs.merchant_containers.insert(FormId::FIRST_DYNAMIC, vec![FormId::PLAYER_REF]);
    let mut w = world(&forms, &refs);
    w.place(BOX, None, [0.0; 3], [0.0; 3], 1).unwrap();
    w.advance_hours(0.5).unwrap();
    // Initial stock was valid; a session edit makes the next restock fail after
    // store_calendar has written globals. The existing calendar policy is not changed.
    w.base_edits.insert(BOX, vec![Entry::new(CYCLE, 1)]);
    assert!(w.advance_hours(48.0).is_err());
    assert_eq!(w.accepted_game_hours(), Some(0.5));
    w.set_global(DAYS, 6.98).unwrap(); w.set_global(HOUR, 23.0).unwrap();
    assert!(w.advance_to_hour(1.0).is_err());
    assert_eq!(w.accepted_game_hours(), Some(0.5));
}

#[test]
fn reserved_inventory_rejects_ordinary_mutation_and_deletion_without_changes() {
    let forms = forms(); let refs = RefTable::default(); let mut w = world(&forms, &refs);
    let id = w.place(BOX, None, [0.0; 3], [0.0; 3], 1).unwrap();
    w.get(id).unwrap().extra.owner = Some(FormId::PLAYER_BASE);
    w.inventory_reservations.insert(id, 7);
    let saved = w.snapshot(); let rng = w.rng.clone(); let serial = w.presence_serial();
    assert_eq!(w.inventory_reservation(id), Some(7));
    assert!(matches!(w.inventory_mut(id), Err(SimError::ReservedInventory(found)) if found == id));
    assert_eq!(w.remove_item(id, BOX, 1), Err(SimError::ReservedInventory(id)));
    assert_eq!(w.set_deleted(id, true), Err(SimError::ReservedInventory(id)));
    assert_eq!(w.set_deleted(id, false), Err(SimError::ReservedInventory(id)));
    assert_eq!(w.snapshot(), saved); assert_eq!(w.rng, rng); assert_eq!(w.presence_serial(), serial);
    let restored = World::restore(&forms, &refs, &saved).unwrap();
    assert_eq!(restored.inventory_reservation(id), Some(7));
}

#[test]
fn reservation_restore_rejects_wrong_token_owner_kind_presence_or_base_policy() {
    for case in 0..11 {
        let mut forms = forms(); let refs = RefTable::default(); let mut w = world(&forms, &refs);
        let id = w.place(BOX, None, [0.0; 3], [0.0; 3], 1).unwrap();
        w.get(id).unwrap().extra.owner = Some(FormId::PLAYER_BASE);
        w.inventory_reservations.insert(id, 7);
        let mut saved = w.snapshot(); drop(w);
        match case {
            0 => { saved.inventory_reservations.insert(id, 0); }
            1 => { saved.refs.get_mut(&id).unwrap().extra.owner = None; }
            2 => { saved.refs.get_mut(&id).unwrap().extra.owner = Some(FormId::PLAYER_REF); }
            3 => { saved.refs.get_mut(&id).unwrap().deleted = true; }
            4 => { saved.refs.get_mut(&id).unwrap().inventory = None; }
            5 => { saved.inventory_reservations.insert(FormId(0xABC), 1); saved.inventory_reservations.remove(&id); }
            6 => {
                let BaseData::Container(container) = &mut forms.bases.get_mut(&BOX).unwrap().data else { unreachable!() };
                container.respawns = true;
            }
            7 => { forms.bases.get_mut(&BOX).unwrap().script = Some(FormId(0xABC)); }
            8 => { saved.inventory_reservations.insert(FormId(id.0 + 1), 7); saved.inventory_reservations.remove(&id); }
            9 => { saved.refs.get_mut(&id).unwrap().base = CYCLE; }
            10 => { saved.deleted_created.insert(id, BOX); }
            _ => unreachable!(),
        }
        assert!(World::restore(&forms, &refs, &saved).is_err(), "invalid reservation case {case}");
    }
}
