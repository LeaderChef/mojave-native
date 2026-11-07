//! Loaded GMST damage-floor behavior, including genuine zero, against armour.
use fnv_combat::{CombatForms, CombatTuning, Rules, data::WeaponStats, damage::{self, Attacker, Target, HitInput, HitPart, NoPerks}};
use fnv_sim::settings::Setting;

#[test]
fn gmst_consumer_loaded_float_changes_damage_floor_and_zero_is_not_missing() {
    let weapon = WeaponStats { base_damage: 20, anim_type: Some("Pistol - Ballistic (1 Hand)".into()), ..WeaponStats::default() };
    let input = HitInput { weapon: &weapon, projectiles: 1, condition: 1.0, ammo_effects: &[], part: HitPart::default(), power: 1.0 };
    let attacker = Attacker { skill: 100.0, ..Attacker::default() };
    let target = Target { dt: 30.0, ..Target::default() };
    for (loaded, expected_floor, expected_damage) in [
        (None, 0.2, 4.0),
        (Some(Setting::Float(0.2)), 0.2, 4.0),
        (Some(Setting::Float(0.0)), 0.0, 0.0),
        (Some(Setting::Float(0.5)), 0.5, 10.0),
        (Some(Setting::Int(0)), 0.2, 4.0),
        (Some(Setting::Float(f32::NAN)), 0.2, 4.0),
    ] {
        let mut forms = CombatForms::default();
        if let Some(value) = loaded { forms.settings.insert("fmindammultiplier".into(), value); }
        let tuning = CombatTuning::from_forms(&forms);
        let hit = damage::hit(&Rules::from_forms(&forms), &tuning, &input, &attacker, &target, &NoPerks, &NoPerks, 0.99);
        assert_eq!(tuning.dt_floor, expected_floor);
        assert!((hit.dealt - expected_damage).abs() < 1e-5, "floor={expected_floor}, {hit:?}");
    }
}
