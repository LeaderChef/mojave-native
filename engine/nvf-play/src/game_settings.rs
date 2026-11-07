//! Fallible, once-per-startup GMST configuration. This consumes the actual VFS,
//! never a reference modlist or a lower-priority fallback file.

use bevy::prelude::Resource;
use fnv_combat::{data::CombatForms, tuning::Rules};
use fnv_sim::settings::{GameSettingsBaseline, Setting, SettingsIni};
use fnv_vfs::Vfs;
use std::io::Read;
use std::collections::BTreeMap;

#[derive(Resource)]
pub struct StartupGameSettings(pub GameSettingsBaseline);

/// Only individually evidenced native defaults supplement actual loaded records.
/// Real records remain authoritative, including their exact type. No-config callers
/// keep the old map. Three spread and six presentation f32 contracts plus the
/// existing documented tagged-skill integer contract supplement it. Unknown keys fail.
pub(crate) fn baseline(forms: &fnv_sim::FormStore, documents: &[SettingsIni]) -> Result<GameSettingsBaseline, String> {
    if documents.is_empty() { return GameSettingsBaseline::parse(&forms.settings, documents); }
    let mut typed: BTreeMap<String, Setting> = fnv_combat::tuning::BUILTIN_SPREAD_SETTINGS.into_iter()
        .chain(crate::presentation_settings::BUILTIN_PRESENTATION_SETTINGS)
        .map(|(name, value)| (name.to_ascii_lowercase(), Setting::Float(value))).collect();
    typed.extend(fnv_game::levelup::BUILTIN_LEVELUP_SETTINGS.into_iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), Setting::Int(value))));
    typed.extend(forms.settings.iter().map(|(name, value)| (name.clone(), value.clone())));
    let baseline = GameSettingsBaseline::parse(&typed, documents)?;
    crate::presentation_settings::validate(&baseline)?;
    Ok(baseline)
}

const MAIN: &str = "nvse\\plugins\\nvse_stewie_tweaks.ini";
const DIRECTORY: &str = "nvse\\plugins\\tweaks\\gamesettings\\";
const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_TOTAL: usize = 64 * 1024 * 1024;

fn is_candidate(path: &str) -> bool {
    path == MAIN || path.strip_prefix(DIRECTORY).is_some_and(|name| !name.contains('\\') && name.ends_with(".ini"))
}

/// A sorted inventory is for reproducible diagnostics only. Conflicting values
/// are rejected by the parser; this does not assert alphabetical load precedence.
pub fn documents(vfs: &Vfs) -> Result<Vec<SettingsIni>, String> {
    let paths = vfs.paths(is_candidate);
    let mut result = Vec::new();
    let mut root = None;
    let mut total = 0;
    for path in paths {
        let Some(file) = vfs.loose_path(&path) else { continue; };
        let root = match &root {
            Some(root) => root,
            None => root.insert(vfs.data_dir().canonicalize().map_err(|e| format!("{}: {e}", vfs.data_dir().display()))?),
        };
        let canonical = file.canonicalize().map_err(|e| format!("{}: {e}", file.display()))?;
        if !canonical.starts_with(root) { return Err(format!("{path}: loose configuration resolves outside Data: {}", canonical.display())); }
        let file = std::fs::File::open(&canonical).map_err(|e| format!("{path}: {e}"))?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| format!("{path}: {e}"))?;
        if bytes.len() > MAX_FILE as usize { return Err(format!("{path}: GameSettings INI exceeds {MAX_FILE} bytes")); }
        total += bytes.len();
        if total > MAX_TOTAL { return Err(format!("GameSettings INIs exceed {MAX_TOTAL} bytes combined")); }
        let text = String::from_utf8(bytes).map_err(|e| format!("{path}: expected UTF-8 INI: {e}"))?;
        result.push(SettingsIni { source: format!("{path} ({})", canonical.display()), text });
    }
    Ok(result)
}

/// No files means exactly the existing simulation/combat construction path.
pub fn prepare(vfs: &Vfs, data: crate::PlayData) -> Result<Option<(crate::sim::SimState, StartupGameSettings)>, String> {
    let documents = documents(vfs)?;
    if documents.is_empty() { return Ok(None); }
    let (simulation, baseline) = crate::sim::SimState::with_settings(data.lo, data.schema, data.index, &documents)?;
    Ok(Some((simulation, StartupGameSettings(baseline))))
}

pub(crate) fn combat_rules(forms: &mut CombatForms, baseline: Option<&StartupGameSettings>) -> Rules {
    if let Some(baseline) = baseline { forms.settings = baseline.0.settings().clone(); }
    Rules::from_forms(forms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fnv_sim::{FormStore, RefTable, SimConfig, World, settings::Setting};

    #[test]
    fn presentation_gmst_six_real_consumers_and_restore_layers() {
        use crate::presentation_settings::PresentationSettings;
        use crate::flashlight::{KeyOutcome, MenuKeyHold};
        let mut forms = FormStore::default();
        assert!(baseline(&forms, &[]).unwrap().settings().is_empty());
        let refs = RefTable::default();
        assert_eq!(PresentationSettings::from_world(&World::new(&forms, &refs, SimConfig::default())), PresentationSettings::default());
        let docs = [SettingsIni { source: "selected-main.ini".into(), text: "[GameSettings]\nfPlayerPipBoyLightTimer=0.5\nfChaseCameraMax=800\nfWorldMapMinZoom=0.1\nfWorldMapMaxZoom=15\nfLocalMapMinZoom=0.2\nfLocalMapMaxZoom=12".into() }];
        let baseline = super::baseline(&forms, &docs).unwrap();
        forms.settings = baseline.settings().iter().map(|(name, value)| (name.clone(), value.clone())).collect();
        let mut world = World::new(&forms, &refs, SimConfig::default());
        let p = PresentationSettings::from_world(&world);
        assert_eq!((p.light_hold, p.chase_max, p.world_map, p.local_map), (0.5, 800.0, (0.1, 15.0), (0.2, 12.0)));
        let mut hold = MenuKeyHold::default();
        assert_eq!(hold.step_with_hold(true, true, 0.49, true, p.light_hold), KeyOutcome::Nothing);
        assert_eq!(hold.step_with_hold(true, false, 0.01, true, p.light_hold), KeyOutcome::ToggleLight);
        assert_eq!(hold.step_with_hold(false, false, 0.01, true, p.light_hold), KeyOutcome::Nothing);
        assert_eq!(crate::control::zoom_with_limit(170.0, -100.0, p.chase_max), 800.0);
        assert_eq!(crate::control::zoom_with_limit(170.0, 100.0, p.chase_max), crate::control::MIN_DISTANCE);
        assert_eq!(crate::pipboy::data::map_zoom(1.0, 100.0, p.world_map), 15.0);
        assert_eq!(crate::pipboy::data::map_zoom(1.0, -100.0, p.local_map), 0.2);
        // Combat and World still receive exactly the same complete baseline.
        let mut combat = CombatForms::default();
        combat_rules(&mut combat, Some(&StartupGameSettings(baseline.clone())));
        assert_eq!(combat.settings["fchasecameramax"], Setting::Float(p.chase_max));
        world.change_setting("fPlayerPipBoyLightTimer", Setting::Float(0.25)).unwrap();
        let saved = world.snapshot();
        let mut restored = World::restore(&forms, &refs, &saved).unwrap();
        assert_eq!(PresentationSettings::from_world(&restored).light_hold, 0.5);
        restored.put_session(world.take_session());
        assert_eq!(PresentationSettings::from_world(&restored).light_hold, 0.25);
        let mut cold_profile = FormStore::default();
        cold_profile.settings.insert("fplayerpipboylighttimer".into(), Setting::Float(0.75));
        let cold = World::restore(&cold_profile, &refs, &saved).unwrap();
        assert_eq!(PresentationSettings::from_world(&cold).light_hold, 0.75);
        // A real loaded record's Float value survives when the INI does not set it.
        let record = super::baseline(&cold_profile, &[SettingsIni { source: "chase.ini".into(), text: "[GameSettings]\nfChaseCameraMax=900".into() }]).unwrap();
        assert_eq!(record.settings()["fplayerpipboylighttimer"], Setting::Float(0.75));
    }

    #[test]
    fn presentation_gmst_invalid_contract_is_atomic_and_unknowns_stay_errors() {
        let forms = FormStore::default();
        for text in [
            "fPlayerPipBoyLightTimer=-0.5",
            "fChaseCameraMax=59",
            "fWorldMapMinZoom=0",
            "fLocalMapMinZoom=16\nfLocalMapMaxZoom=15",
            "fFirstPersonHandFollowMult=1",
            "fPainDelay=6000",
            "iMerchantRespawnDay2=1.5",
        ] {
            let docs = [SettingsIni { source: "invalid.ini".into(), text: format!("[GameSettings]\n{text}") }];
            let error = baseline(&forms, &docs).unwrap_err();
            assert!(error.contains("invalid.ini:"), "{error}");
            assert!(forms.settings.is_empty());
        }
        let mut wrong = FormStore::default();
        wrong.settings.insert("fchasecameramax".into(), Setting::Int(450));
        let error = baseline(&wrong, &[SettingsIni { source: "record-type.ini".into(), text: "[GameSettings]\nfChaseCameraMax=800".into() }]).unwrap_err();
        assert!(error.contains("record-type.ini:2") && error.contains("requires a finite Float"), "{error}");
        assert_eq!(wrong.settings["fchasecameramax"], Setting::Int(450));
    }

    #[test]
    fn gmst_consumer_three_defaults_record_types_and_world_combat_agree() {
        let mut forms = FormStore::default();
        forms.settings.insert("fmindammultiplier".into(), Setting::Float(0.2));
        assert_eq!(baseline(&forms, &[]).unwrap().settings().len(), 1);
        let docs = [SettingsIni { source: "real-winner.ini".into(), text: "[GameSettings]\nfCrippledArms1HSpreadPenalty=0.7\nfCrippledArms2HSpreadPenalty=0.9\nfRunningSpreadPenalty=0.5\nfMinDamMultiplier=0".into() }];
        let snapshot = baseline(&forms, &docs).unwrap();
        forms.settings = snapshot.settings().iter().map(|(name, value)| (name.clone(), value.clone())).collect();
        let refs = RefTable::default();
        let world = World::new(&forms, &refs, SimConfig::default());
        let mut combat = CombatForms::default();
        let rules = combat_rules(&mut combat, Some(&StartupGameSettings(snapshot)));
        let tuning = fnv_combat::CombatTuning::from_forms(&combat);
        for (name, value) in [("fCrippledArms1HSpreadPenalty", rules.spread_crippled_arms_1h), ("fCrippledArms2HSpreadPenalty", rules.spread_crippled_arms_2h), ("fRunningSpreadPenalty", rules.spread_running), ("fMinDamMultiplier", tuning.dt_floor)] {
            assert_eq!(world.setting_f32(name), Some(value), "{name}");
        }
        assert_eq!((rules.spread_crippled_arms_1h, rules.spread_crippled_arms_2h, rules.spread_running, tuning.dt_floor), (0.7, 0.9, 0.5, 0.0));
        // A real record overrides registry metadata, not only its default value.
        let mut wrong_record = FormStore::default();
        wrong_record.settings.insert("frunningspreadpenalty".into(), Setting::Int(2));
        assert!(baseline(&wrong_record, &docs).unwrap_err().contains("real-winner.ini:4"));
        for unknown in ["iUnproven=2", "iMerchantRespawnDay2=1.5", "fUnproven=1"] {
            let invalid = [SettingsIni { source: "unknown.ini".into(), text: format!("[GameSettings]\n{unknown}") }];
            assert!(baseline(&forms, &invalid).unwrap_err().contains("unknown.ini:2"));
        }
    }

    #[test]
    fn tagged_skill_gmst_exact_integer_registration_and_record_precedence() {
        let forms = FormStore::default();
        assert!(baseline(&forms, &[]).unwrap().settings().is_empty());
        let docs = [SettingsIni { source: "Hardercore.ini".into(), text: "[GameSettings]\niSkillPointsTagSkillMult=2".into() }];
        let selected = baseline(&forms, &docs).unwrap();
        assert_eq!(selected.settings()["iskillpointstagskillmult"], Setting::Int(2));
        let mut real = FormStore::default();
        real.settings.insert("iskillpointstagskillmult".into(), Setting::Int(3));
        let other = [SettingsIni { source: "chase.ini".into(), text: "[GameSettings]\nfChaseCameraMax=800".into() }];
        assert_eq!(baseline(&real, &other).unwrap().settings()["iskillpointstagskillmult"], Setting::Int(3));
        for bad in ["iSkillPointsTagSkillMult=2.5", "iSkillPointsTagSkillMult=2147483648", "iUnproven=2"] {
            let bad = [SettingsIni { source: "bad.ini".into(), text: format!("[GameSettings]\n{bad}") }];
            assert!(baseline(&forms, &bad).unwrap_err().contains("bad.ini:2"));
        }
    }

    #[test]
    fn startup_gmst_world_combat_and_restore_layers() {
        let mut forms = FormStore::default();
        forms.settings.insert("fdamageskillbase".into(), Setting::Float(0.5));
        let baseline = GameSettingsBaseline::parse(&forms.settings, &[SettingsIni {
            source: "winning.ini".into(), text: "[GameSettings]\nfDamageSkillBase=0.75".into(),
        }]).unwrap();
        forms.settings = baseline.settings().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        let refs = RefTable::default();
        let mut world = World::new(&forms, &refs, SimConfig::default());
        let mut combat = CombatForms::default();
        let rules = combat_rules(&mut combat, Some(&StartupGameSettings(baseline.clone())));
        assert_eq!(world.setting_f32("fDamageSkillBase"), Some(rules.damage_skill_base));
        assert_eq!(rules.damage_skill_base, 0.75);
        assert_eq!(combat.settings.iter().collect::<Vec<_>>(), baseline.settings().iter().collect::<Vec<_>>());
        world.change_setting("fDamageSkillBase", Setting::Float(0.9)).unwrap();
        let snapshot = world.snapshot();
        let mut restored = World::restore(&forms, &refs, &snapshot).unwrap();
        assert_eq!(restored.setting_f32("fDamageSkillBase"), Some(0.75));
        restored.put_session(world.take_session());
        assert_eq!(restored.setting_f32("fDamageSkillBase"), Some(0.9));
        assert_eq!(baseline.settings()["fdamageskillbase"], Setting::Float(0.75));
        let mut other_profile = FormStore::default();
        other_profile.settings.insert("fdamageskillbase".into(), Setting::Float(0.6));
        let cold = World::restore(&other_profile, &refs, &snapshot).unwrap();
        assert_eq!(cold.setting_f32("fDamageSkillBase"), Some(0.6));
    }

    #[test]
    fn startup_gmst_no_config_preserves_combat_defaults() {
        let mut combat = CombatForms::default();
        combat.settings.insert("fdamageskillbase".into(), Setting::Float(0.4));
        let before = combat.settings.clone();
        let rules = combat_rules(&mut combat, None);
        assert_eq!(combat.settings, before);
        assert_eq!(rules.damage_skill_base, 0.4);
        let root = std::env::temp_dir().join(format!("nvf-startup-gmst-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let vfs = Vfs::open(&root, &[]).unwrap();
        assert!(documents(&vfs).unwrap().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn startup_gmst_only_resolved_loose_candidate_files() {
        let root = std::env::temp_dir().join(format!("nvf-startup-gmst-files-{}", std::process::id()));
        let folder = root.join("nvse/plugins/tweaks/GameSettings");
        std::fs::create_dir_all(folder.join("nested")).unwrap();
        std::fs::write(root.join("nvse/plugins/nvse_stewie_tweaks.ini"), "[GameSettings]\nfA=1").unwrap();
        std::fs::write(folder.join("winner.ini"), "[GameSettings]\nfB=2").unwrap();
        std::fs::write(folder.join("ignore.txt"), "ignored").unwrap();
        std::fs::write(folder.join("nested/ignored.ini"), "ignored").unwrap();
        let vfs = Vfs::open(&root, &[]).unwrap();
        let docs = documents(&vfs).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|doc| doc.source.contains("nvse_stewie_tweaks.ini")));
        #[cfg(unix)] {
            let outside = root.with_extension("outside.ini");
            std::fs::write(&outside, "[GameSettings]").unwrap();
            std::os::unix::fs::symlink(&outside, folder.join("escape.ini")).unwrap();
            let vfs = Vfs::open(&root, &[]).unwrap();
            assert!(documents(&vfs).unwrap_err().contains("outside Data"));
            std::fs::remove_file(outside).unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
