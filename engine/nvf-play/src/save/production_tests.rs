//! Real body-reader boundaries over empty synthetic game state, never game assets.

use super::*;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

fn body() -> Value {
    let script = crate::script::ScriptState::over_no_plugins();
    serde_json::to_value(SaveBody {
        format: BODY_FORMAT.into(),
        game: script.rt.as_ref().unwrap().game.snapshot(),
        engine: EngineSnapshot { format: ENGINE_FORMAT.into(), ..default() },
    }).unwrap()
}

fn legacy_body() -> Value {
    let mut raw = body();
    raw["format"] = LEGACY_BODY_FORMAT.into();
    raw["game"]["format"] = fnv_game::game::LEGACY_SNAPSHOT_FORMAT.into();
    raw["game"].as_object_mut().unwrap().remove("production");
    raw
}

struct TempSave { path: PathBuf, bytes: Vec<u8> }

impl TempSave {
    fn new(raw: &Value, plugins: Vec<PluginEntry>) -> Self {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("nvf-production-reader-{}-{serial}.nvfsave", std::process::id()));
        let mut header = SaveHeader::new(SaveKind::Manual);
        header.plugins = plugins;
        let bytes = fnv_save::encode(&header, &[], &serde_json::to_vec(raw).unwrap());
        std::fs::write(&path, &bytes).unwrap();
        Self { path, bytes }
    }

    fn unchanged(&self) {
        assert_eq!(std::fs::read(&self.path).unwrap(), self.bytes, "a reader must not rewrite a rejected save");
    }
}

impl Drop for TempSave {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); }
}

fn plugins() -> Vec<PluginEntry> {
    vec![PluginEntry { name: "FalloutNV.esm".into(), size: 10 }, PluginEntry { name: "Orders.esm".into(), size: 20 }]
}

#[test]
fn legacy_no_job_body_reads_without_seeding_inventory_or_orders() {
    let raw = legacy_body();
    let saved = TempSave::new(&raw, vec![]);
    let (_, loaded, order) = read_for_load(&saved.path, &[]).unwrap();
    assert!(order.is_none());
    assert_eq!(loaded.format, LEGACY_BODY_FORMAT);
    assert!(loaded.game.production.is_pristine());
    assert_eq!(serde_json::to_value(loaded.game.sim).unwrap(), raw["game"]["sim"]);
    saved.unchanged();
}

#[test]
fn current_body_requires_the_explicit_ledger_and_roundtrips_it() {
    let mut raw = body();
    let saved = TempSave::new(&raw, vec![]);
    let (_, loaded, _) = read_for_load(&saved.path, &[]).unwrap();
    assert_eq!(loaded.format, BODY_FORMAT);
    assert_eq!(serde_json::to_value(loaded.game.production).unwrap(), raw["game"]["production"]);
    raw["game"].as_object_mut().unwrap().remove("production");
    let absent = TempSave::new(&raw, vec![]);
    assert!(read_for_load(&absent.path, &[]).unwrap_err().contains("ledger; it is missing"));
    saved.unchanged(); absent.unchanged();
}

#[test]
fn mixed_legacy_current_and_future_formats_are_refused_before_restore() {
    let current = fnv_game::game::SNAPSHOT_FORMAT;
    let legacy = fnv_game::game::LEGACY_SNAPSHOT_FORMAT;
    for (bf, gf) in [(LEGACY_BODY_FORMAT, current), (BODY_FORMAT, legacy), ("nvf.save.body/999", current), (BODY_FORMAT, "fnv.game.state/999")] {
        let mut raw = body();
        raw["format"] = bf.into(); raw["game"]["format"] = gf.into();
        let saved = TempSave::new(&raw, vec![]);
        assert!(read_for_load(&saved.path, &[]).unwrap_err().contains("inconsistent save formats"));
        saved.unchanged();
    }
}

#[test]
fn malformed_and_unknown_ledger_shapes_are_not_defaulted_to_empty() {
    let mut unknown = body()["game"]["production"].clone();
    unknown.as_object_mut().unwrap().insert("future_grant".into(), json!(123));
    for invalid in [Value::Null, json!([]), json!("not a ledger"), unknown] {
        let mut raw = body(); raw["game"]["production"] = invalid;
        let saved = TempSave::new(&raw, vec![]);
        assert!(read_for_load(&saved.path, &[]).unwrap_err().contains("invalid production ledger"));
        saved.unchanged();
    }
}

#[test]
fn future_ledger_schema_is_refused_even_when_no_orders_are_present() {
    let mut raw = body();
    raw["game"]["production"]["schema"] = json!("j6.production/future");
    let saved = TempSave::new(&raw, vec![]);
    assert!(read_for_load(&saved.path, &[]).unwrap_err().contains("invalid production ledger"));
    saved.unchanged();
}

#[test]
fn raw_gate_runs_before_generic_remap_can_drop_ledger_entries() {
    let mut raw = body();
    // The ordinary walker drops tuples containing the now-missing Orders.esm
    // form. The typed gate must reject this invalid ledger instead of erasing it.
    raw["game"]["production"] = json!([["0x01000100", "unrecognised receipt"]]);
    let saved = TempSave::new(&raw, plugins());
    let current = vec![plugins().remove(0)];
    assert!(read_for_load(&saved.path, &current).unwrap_err().contains("invalid production ledger"));
    saved.unchanged();
}

#[test]
fn ledger_history_keeps_plugin_prefix_sizes_and_allows_only_appending() {
    let saved = plugins();
    assert!(production_load_order(true, &saved, &saved).is_ok());
    let mut appended = saved.clone(); appended.push(PluginEntry { name: "Extra.esm".into(), size: 30 });
    assert!(production_load_order(true, &saved, &appended).is_ok());
    let mut case = saved.clone(); case[0].name = "falloutnv.ESM".into();
    assert!(production_load_order(true, &saved, &case).is_ok());
    let mut reordered = saved.clone(); reordered.swap(0, 1);
    let removed = saved[..1].to_vec();
    let mut resized = saved.clone(); resized[1].size += 1;
    for changed in [reordered, removed, resized] {
        assert!(production_load_order(true, &saved, &changed).is_err());
        assert!(production_load_order(false, &saved, &changed).is_ok(), "a pristine book has no financial remap rights to lose");
    }
}

#[test]
fn actual_body_reader_preserves_sticky_history_and_rejects_its_plugin_changes() {
    let mut raw = body();
    // A retained revision is meaningful even with no active order. It must not
    // be silently treated as a pristine book after a job has closed.
    raw["game"]["production"]["revision"] = json!(1);
    let saved_plugins = plugins();
    let saved = TempSave::new(&raw, saved_plugins.clone());
    let mut appended = saved_plugins.clone(); appended.push(PluginEntry { name: "Extra.esm".into(), size: 30 });
    let (_, loaded, _) = read_for_load(&saved.path, &appended).unwrap();
    assert!(!loaded.game.production.is_pristine());
    assert_eq!(serde_json::to_value(loaded.game.production).unwrap(), raw["game"]["production"]);
    let mut reordered = saved_plugins.clone(); reordered.swap(0, 1);
    let removed = saved_plugins[..1].to_vec();
    let mut resized = saved_plugins; resized[1].size += 1;
    for changed in [reordered, removed, resized] {
        assert!(read_for_load(&saved.path, &changed).unwrap_err().contains("require the saved plugin order and sizes"));
        saved.unchanged();
    }
}

#[test]
fn legacy_format_cannot_smuggle_a_nonpristine_book_past_the_old_reader_gate() {
    let mut raw = legacy_body();
    let mut book = serde_json::to_value(fnv_game::production::ProductionBook::default()).unwrap();
    book["revision"] = json!(1);
    raw["game"]["production"] = book;
    let saved = TempSave::new(&raw, vec![]);
    assert!(read_for_load(&saved.path, &[]).unwrap_err().contains("legacy save cannot contain production orders or receipts"));
    saved.unchanged();
}

#[test]
fn pristine_current_body_keeps_the_ordinary_plugin_remap_path() {
    let raw = body();
    let saved_plugins = plugins();
    let saved = TempSave::new(&raw, saved_plugins.clone());
    let mut reordered = saved_plugins; reordered.swap(0, 1);
    let (_, loaded, order) = read_for_load(&saved.path, &reordered).unwrap();
    assert!(order.is_some());
    assert!(loaded.game.production.is_pristine());
    saved.unchanged();
}
