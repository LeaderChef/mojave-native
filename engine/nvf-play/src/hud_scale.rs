//! Optional compact corner HUD, independent of menu readability and aiming.
//! The original size remains 1.0; replacement tile HUDs retain their own layout.

use bevy::prelude::*;
use crate::interface::u;
use crate::settings::{add_setting, Setting, Slider};

#[derive(Resource, Clone, Copy, Debug)]
pub struct HudScale(pub f32);

impl Default for HudScale {
    fn default() -> Self { Self(1.0) }
}

/// The translation that would retain the chosen anchor at zero scale, in the
/// interface's 960-height units. All descendants, including compass marks, follow.
#[derive(Component)]
pub(crate) struct ScaledPiece(pub Vec2);

pub(crate) fn register(app: &mut App) {
    app.init_resource::<HudScale>();
    add_setting(app, Setting::slider(
        "hud-scale", "Display", "HUD size",
        "Size of the corner meters, compass, needs and enemy health. Menus and aiming stay unchanged. Replacement HUD mods keep their own layout.",
        Slider { get: |w| w.resource::<HudScale>().0,
            set: |w, v| w.resource_mut::<HudScale>().0 = v,
            min: 0.6, step: 0.05, shown: None },
    ));
}

pub(crate) fn apply(scale: Res<HudScale>, mut pieces: Query<(Ref<ScaledPiece>, &mut UiTransform)>) {
    // No tree walk, respawn or texture allocation when the slider is unchanged.
    for (piece, mut transform) in &mut pieces {
        if !scale.is_changed() && !piece.is_added() { continue; }
        let factor = if scale.0.is_finite() { scale.0.clamp(0.6, 1.0) } else { 1.0 };
        transform.scale = Vec2::splat(factor);
        let offset = piece.0 * (1.0 - factor);
        transform.translation = Val2::new(u(offset.x), u(offset.y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn compact_hud_keeps_corner_and_need_line_together_without_scaling_menus() {
        let mut app = App::new();
        register(&mut app);
        app.add_systems(Update, apply);
        let hp = app.world_mut().spawn((ScaledPiece(Vec2::new(-184.5, 63.5)), UiTransform::default())).id();
        // Needs start 150 right of the HP corner; its bottom is 100 units higher.
        let needs = app.world_mut().spawn((ScaledPiece(Vec2::new(-343.0, 110.0)), UiTransform::default())).id();
        let menu = app.world_mut().spawn(UiTransform::default()).id();
        app.world_mut().resource_mut::<HudScale>().0 = 0.8;
        app.update();
        let h = app.world().get::<UiTransform>(hp).unwrap();
        let n = app.world().get::<UiTransform>(needs).unwrap();
        let vh = |v| if let Val::Vh(x) = v { x * 9.6 } else { panic!("viewport-relative offset required") };
        // UiTransform scales about each node's center. The correction preserves
        // the outer HP corner; the needs' baseline then follows the same pivot.
        assert!((vh(h.translation.x) + 184.5 * 0.2).abs() < 0.001);
        assert!((vh(h.translation.y) - 63.5 * 0.2).abs() < 0.001);
        assert!((150.0 + 193.0 * 0.2 + vh(n.translation.x) - 150.0 * 0.8).abs() < 0.001);
        assert_eq!(app.world().get::<UiTransform>(menu).unwrap().scale, Vec2::ONE);
        // A late-created HUD piece receives the current setting too.
        let late = app.world_mut().spawn((ScaledPiece(Vec2::ZERO), UiTransform::default())).id();
        app.update();
        assert_eq!(app.world().get::<UiTransform>(late).unwrap().scale, Vec2::splat(0.8));
        let settings = app.world().resource::<Settings>();
        assert!(settings.index("hud-scale").is_some());
    }
}
