//! Authored damage consumers for the native optional-patch contract; no game data.
use fnv_combat::{data::{AmmoEffect, WeaponStats}, damage::{self, Attacker, HitInput, HitPart, NoPerks, PerkHooks, Target}, CombatTuning, Rules};

fn hit(w: &WeaponStats, target: &Target, ammo: &[&AmmoEffect], perks: &dyn PerkHooks) -> damage::Damage {
    damage::hit(&Rules::defaults(), &CombatTuning::default(),
        &HitInput { weapon: w, projectiles: 1, condition: 1.0, ammo_effects: ammo, part: HitPart::default(), power: 1.0 },
        &Attacker { skill: 100.0, unarmed_skill: 100.0, ..Default::default() }, target, perks, &NoPerks, 0.99)
}

#[test]
fn automatic_melee_and_unarmed_ignore_armor_only_when_the_weapon_flags_it() {
    let target = Target { dt: 15.0, dr: 50.0, ..Default::default() };
    for anim in ["Melee (1 Hand)", "Melee (2 Hand)", "Hand to Hand"] {
        let mut w = WeaponStats { base_damage: 40, anim_type: Some(anim.into()), automatic: true, ..Default::default() };
        let ordinary = hit(&w, &target, &[], &NoPerks);
        let naked = hit(&w, &Target::default(), &[], &NoPerks);
        assert!(ordinary.dealt < naked.dealt);
        assert_eq!((ordinary.dt, ordinary.dr), (15.0, 50.0));
        w.flags.push("Ignores Normal Weapon Resistance".into());
        let flagged = hit(&w, &target, &[], &NoPerks);
        assert_eq!((flagged.dt, flagged.dr), (0.0, 0.0));
        assert_eq!(flagged.dealt, naked.dealt, "{anim}");
        let immune = hit(&w, &Target { child: true, ..target.clone() }, &[], &NoPerks);
        assert_eq!(immune.dealt, 0.0, "the armor flag must not bypass unrelated target protection");
    }
}

struct Bonus;
impl PerkHooks for Bonus {
    fn weapon_damage(&self, v: f32) -> f32 { v * 1.25 }
}

#[test]
fn ammo_and_perks_precede_dt_then_dr_and_ignore_flag_preserves_their_damage() {
    let mut w = WeaponStats { base_damage: 20, anim_type: Some("Pistol - Ballistic (1 Hand)".into()), ..Default::default() };
    let ammo = AmmoEffect { kind: Some("Damage Mod".into()), operation: Some("Multiply".into()), value: 1.5, ..Default::default() };
    let target = Target { dt: 10.0, dr: 50.0, ..Default::default() };
    let d = hit(&w, &target, &[&ammo], &Bonus);
    // 20 * 1.5 * 1.25 = 37.5, then (37.5 - 10) * 0.5 = 13.75.
    assert_eq!((d.raw, d.dealt), (37.5, 13.75));
    assert_ne!(d.dealt, d.raw * 0.5 - 10.0, "DR-before-DT must not be substituted");
    w.flags.push("Ignores Normal Weapon Resistance".into());
    let piercing = hit(&w, &target, &[&ammo], &Bonus);
    assert_eq!((piercing.raw, piercing.dealt), (37.5, 37.5));
}
