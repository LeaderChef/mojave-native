//! Game settings (GMST): named engine constants authored in the plugins.
//!
//! GMST DATA is a union decided by the first letter of the Editor ID: `f` → f32,
//! `i` → i32, `s` → NUL-terminated string [documented: xEdit wbDefinitionsFNV.pas GMST;
//! observed: all 459 `f` and 61 `i` settings in the masters are 4 bytes]. The schema
//! keeps DATA as bytes (schema v1 has no unions) and this module interprets them.

use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};

/// One resolved loose INI, read once before the simulation is constructed.
#[derive(Debug, Clone)]
pub struct SettingsIni {
    pub source: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingOrigin {
    pub source: String,
    pub line: usize,
}

/// Immutable startup values. Runtime/session edits belong to `World`, not here.
#[derive(Debug, Clone)]
pub struct GameSettingsBaseline {
    settings: Arc<BTreeMap<String, Setting>>,
    origins: BTreeMap<String, SettingOrigin>,
}

impl GameSettingsBaseline {
    /// Stage every assignment against the actual winning GMST record's type. A
    /// failure returns no baseline and never mutates `loaded`. File ordering is not
    /// established, so distinct values for one setting are an error, not last-wins.
    pub fn parse<'a>(loaded: impl IntoIterator<Item = (&'a String, &'a Setting)>, documents: &[SettingsIni]) -> Result<Self, String> {
        let loaded: BTreeMap<String, Setting> = loaded.into_iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        let mut settings = loaded.clone();
        let mut origins: BTreeMap<String, SettingOrigin> = BTreeMap::new();
        for document in documents {
            let mut in_settings = false;
            for (index, line) in document.text.trim_start_matches('\u{feff}').lines().enumerate() {
                let line_number = index + 1;
                let origin = || format!("{}:{line_number}", document.source);
                let line = line.trim();
                if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                    continue;
                }
                if line.starts_with('[') {
                    let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
                        return Err(format!("{}: malformed INI section", origin()));
                    };
                    in_settings = section.trim().eq_ignore_ascii_case("GameSettings");
                    continue;
                }
                if !in_settings { continue; }
                let Some((name, raw)) = line.split_once('=') else {
                    return Err(format!("{}: expected setting=value in [GameSettings]", origin()));
                };
                let key = name.trim().to_ascii_lowercase();
                let raw = raw.trim();
                let Some(kind) = loaded.get(&key) else {
                    return Err(format!("{}: {name}: no loaded GMST type; engine-default types are not inferred", origin()));
                };
                let bad = |why: &str| format!("{}: {}: {why}: {raw:?}", origin(), name.trim());
                let value = match kind {
                    Setting::Float(_) => {
                        let parsed = raw.parse::<f64>().map_err(|_| bad("expected finite float"))?;
                        let value = parsed as f32;
                        if !parsed.is_finite() || !value.is_finite() { return Err(bad("non-finite or overflowing float")); }
                        Setting::Float(value)
                    }
                    Setting::Int(_) => Setting::Int(raw.parse::<i32>().map_err(|_| bad("expected exact signed 32-bit integer literal (no fractions or overflow)"))?),
                    Setting::Str(_) => Setting::Str(raw.to_owned()),
                    Setting::Raw(_) => return Err(bad("loaded GMST has unsupported/raw type")),
                };
                if let Some(previous) = origins.get(&key) {
                    if settings.get(&key) != Some(&value) {
                        return Err(format!("{}: {} conflicts with {}:{}; INI file ordering is not established", origin(), name.trim(), previous.source, previous.line));
                    }
                } else {
                    origins.insert(key.clone(), SettingOrigin { source: document.source.clone(), line: line_number });
                    settings.insert(key, value);
                }
            }
        }
        Ok(Self { settings: Arc::new(settings), origins })
    }

    pub fn settings(&self) -> &BTreeMap<String, Setting> { &self.settings }
    pub fn origins(&self) -> &BTreeMap<String, SettingOrigin> { &self.origins }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Setting {
    Float(f32),
    Int(i32),
    Str(String),
    /// A prefix or size the rule above does not cover (reported by the loader).
    Raw(Vec<u8>),
}

impl Setting {
    /// Interpret DATA bytes for a setting with this Editor ID.
    pub fn from_bytes(edid: &str, data: &[u8]) -> Setting {
        match (edid.as_bytes().first(), data.len()) {
            (Some(b'f'), 4) => Setting::Float(f32::from_le_bytes(data.try_into().expect("4 bytes"))),
            (Some(b'i'), 4) => Setting::Int(i32::from_le_bytes(data.try_into().expect("4 bytes"))),
            (Some(b's'), _) => Setting::Str(fnv_util::text::decode_z(data)),
            _ => Setting::Raw(data.to_vec()),
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Setting::Float(v) => Some(v),
            Setting::Int(v) => Some(v as f32),
            _ => None,
        }
    }
}

/// Parse the lower-case hex the schema decoder prints for `bytes` fields.
pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_by_prefix() {
        assert_eq!(Setting::from_bytes("fAVDCarryWeightsBase", &150f32.to_le_bytes()), Setting::Float(150.0));
        assert_eq!(Setting::from_bytes("iLevelCap", &30i32.to_le_bytes()), Setting::Int(30));
        assert_eq!(Setting::from_bytes("sOk", b"Ok\0"), Setting::Str("Ok".into()));
        assert_eq!(Setting::from_bytes("bX", &[1]), Setting::Raw(vec![1]));
        assert_eq!(Setting::from_bytes("fShort", &[0, 0]), Setting::Raw(vec![0, 0]));
        assert_eq!(unhex("00001643"), Some(vec![0, 0, 0x16, 0x43]));
        assert_eq!(unhex("0"), None);
        assert_eq!(unhex("zz"), None);
        assert_eq!(Setting::Int(3).as_f32(), Some(3.0));
    }

    fn loaded() -> BTreeMap<String, Setting> {
        BTreeMap::from([("fdamage".into(), Setting::Float(1.0)), ("icount".into(), Setting::Int(2)), ("slabel".into(), Setting::Str("old".into())), ("raw".into(), Setting::Raw(vec![1]))])
    }

    fn ini(source: &str, body: &str) -> SettingsIni { SettingsIni { source: source.into(), text: format!("[GameSettings]\n{body}") } }

    #[test]
    fn startup_gmst_types_atomicity_and_provenance() {
        let original = loaded();
        let good = GameSettingsBaseline::parse(&original, &[ini("winner.ini", "FDamage=2.5\niCount=7\nsLabel=new")]).unwrap();
        assert_eq!(good.settings()["fdamage"], Setting::Float(2.5));
        assert_eq!(good.settings()["icount"], Setting::Int(7));
        assert_eq!(good.origins()["icount"].line, 3);
        for bad in ["iCount=1.5", "iCount=2147483648", "fDamage=NaN", "fDamage=1e300", "fDamage=inf", "fUnknown=2", "raw=1"] {
            let err = GameSettingsBaseline::parse(&original, &[ini("good.ini", "iCount=9"), ini("bad.ini", bad)]).unwrap_err();
            assert!(err.contains("bad.ini:2"), "{err}");
            assert_eq!(original, loaded());
        }
    }

    #[test]
    fn startup_gmst_conflicts_are_not_file_order() {
        let files = [ini("first.ini", "fDamage=2"), ini("second.ini", "fDamage=3")];
        let err = GameSettingsBaseline::parse(&loaded(), &files).unwrap_err();
        assert!(err.contains("first.ini:2") && err.contains("second.ini:2"));
        let equal = [ini("first.ini", "fDamage=2"), ini("second.ini", "fDamage=2.0")];
        assert!(GameSettingsBaseline::parse(&loaded(), &equal).is_ok());
        assert!(GameSettingsBaseline::parse(&loaded(), &[]).unwrap().origins().is_empty());
    }
}
