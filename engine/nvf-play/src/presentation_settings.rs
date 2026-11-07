//! Six existing native f32 presentation consumers, now configurable by the
//! shared startup baseline. Evidence and native-default policy:
//! `docs/spec-notes/presentation-game-settings.md`.

use fnv_sim::{settings::{GameSettingsBaseline, Setting}, World};

pub const LIGHT_HOLD: f32 = 1.0;
pub const CHASE_MAX: f32 = 450.0;
pub const MAP_ZOOM: (f32, f32) = (1.0, 4.0);

/// Individually named native contracts, not a type inference from key prefixes.
/// The values are the previous native consumer defaults, not original-game values.
pub const BUILTIN_PRESENTATION_SETTINGS: [(&str, f32); 6] = [
    ("fPlayerPipBoyLightTimer", LIGHT_HOLD),
    ("fChaseCameraMax", CHASE_MAX),
    ("fWorldMapMinZoom", MAP_ZOOM.0),
    ("fWorldMapMaxZoom", MAP_ZOOM.1),
    ("fLocalMapMinZoom", MAP_ZOOM.0),
    ("fLocalMapMaxZoom", MAP_ZOOM.1),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationSettings {
    pub light_hold: f32,
    pub chase_max: f32,
    pub world_map: (f32, f32),
    pub local_map: (f32, f32),
}

impl Default for PresentationSettings {
    fn default() -> Self {
        Self { light_hold: LIGHT_HOLD, chase_max: CHASE_MAX, world_map: MAP_ZOOM, local_map: MAP_ZOOM }
    }
}

impl PresentationSettings {
    /// Read the real simulation world, including same-run session overrides. This
    /// does not read ScriptState's empty stand-in world or touch disk.
    pub fn from_world(world: &World<'_>) -> Self {
        let float = |name: &str, fallback| match world.setting_value(name) {
            Some(Setting::Float(value)) if value.is_finite() => *value,
            _ => fallback,
        };
        let d = Self::default();
        let bounds = |min, max| {
            let pair = (float(min, MAP_ZOOM.0), float(max, MAP_ZOOM.1));
            if pair.0 > 0.0 && pair.1 >= pair.0 { pair } else { MAP_ZOOM }
        };
        let hold = float("fPlayerPipBoyLightTimer", d.light_hold);
        let chase = float("fChaseCameraMax", d.chase_max);
        Self {
            light_hold: if hold >= 0.0 { hold } else { d.light_hold },
            chase_max: if chase >= crate::control::MIN_DISTANCE { chase } else { d.chase_max },
            world_map: bounds("fWorldMapMinZoom", "fWorldMapMaxZoom"),
            local_map: bounds("fLocalMapMinZoom", "fLocalMapMaxZoom"),
        }
    }
}

/// Validate authored values before a World or cached combat rules are published.
/// Real-record types remain authoritative: an authored Int cannot be silently
/// reinterpreted as the Float required by these native consumers.
pub(crate) fn validate(baseline: &GameSettingsBaseline) -> Result<(), String> {
    let error = |name: &str, why: &str| {
        let key = name.to_ascii_lowercase();
        match baseline.origins().get(&key) {
            Some(origin) => format!("{}:{}: {name}: {why}", origin.source, origin.line),
            None => format!("loaded GMST {name}: {why}"),
        }
    };
    for (name, _) in BUILTIN_PRESENTATION_SETTINGS {
        if baseline.origins().contains_key(&name.to_ascii_lowercase())
            && !matches!(baseline.settings().get(&name.to_ascii_lowercase()), Some(Setting::Float(value)) if value.is_finite()) {
            return Err(error(name, "native presentation consumer requires a finite Float; loaded record type is authoritative"));
        }
    }
    let float = |name: &str, default| match baseline.settings().get(&name.to_ascii_lowercase()) {
        Some(Setting::Float(value)) if value.is_finite() => *value,
        _ => default,
    };
    if float("fPlayerPipBoyLightTimer", LIGHT_HOLD) < 0.0 {
        return Err(error("fPlayerPipBoyLightTimer", "hold time must be nonnegative"));
    }
    if float("fChaseCameraMax", CHASE_MAX) < crate::control::MIN_DISTANCE {
        return Err(error("fChaseCameraMax", "maximum must be at least the native 60-unit near limit"));
    }
    for (min, max) in [("fWorldMapMinZoom", "fWorldMapMaxZoom"), ("fLocalMapMinZoom", "fLocalMapMaxZoom")] {
        let (lo, hi) = (float(min, MAP_ZOOM.0), float(max, MAP_ZOOM.1));
        if lo <= 0.0 { return Err(error(min, "map zoom minimum must be positive")); }
        if hi < lo { return Err(error(max, "map zoom maximum must be at least its minimum")); }
    }
    Ok(())
}
