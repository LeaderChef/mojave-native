//! Optional, native stock-sensitive barter [design: market-pricing.md].
//! Only price targets are saved here; inventories and caps remain the simulation's.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use super::economy::MarketBook;

#[derive(Resource, Default)]
pub(crate) struct Markets {
    pub enabled: bool,
    pub book: MarketBook,
}

const FORMAT: &str = "j6.market-pricing/1";

#[derive(Serialize, Deserialize)]
struct Saved {
    format: String,
    book: MarketBook,
}

pub(super) fn setup(app: &mut App) {
    app.init_resource::<Markets>();
    crate::settings::add_setting(app, crate::settings::Setting {
        key: "stock-prices",
        group: "Living world",
        label: "Stock-sensitive barter prices",
        note: "Changes prices with each merchant's stock. Applies to the next trade. Does not change restocking or NPC purchases.",
        get: |w| w.resource::<Markets>().enabled,
        set: |w, enabled| w.resource_mut::<Markets>().enabled = enabled,
        slider: None,
    });
    crate::save::add_save_hook(app, crate::save::SaveHook {
        key: "j6_market_pricing", save, load, remap: None,
    });
}

fn save(world: &World) -> Option<serde_json::Value> {
    let state = world.get_resource::<Markets>()?;
    if state.book == MarketBook::default() {
        return None;
    }
    serde_json::to_value(Saved { format: FORMAT.into(), book: state.book.clone() }).ok()
}

fn load(world: &mut World, value: &serde_json::Value) {
    // Engine read_for_load remaps FormId strings throughout the body once. A second
    // remap hook here would move already remapped market/item keys twice.
    let parsed = serde_json::from_value::<Saved>(value.clone()).map_err(|e| e.to_string()).and_then(|s| {
        if s.format != FORMAT {
            return Err(format!("unsupported pricing format {}", s.format));
        }
        s.book.validate()?;
        Ok(s.book)
    });
    let Some(mut state) = world.get_resource_mut::<Markets>() else { return };
    match parsed {
        Ok(book) => {
            state.book = book;
            info!("market: restored price targets; inventories and caps stay in the world save");
        }
        Err(e) => {
            state.enabled = false;
            state.book = MarketBook::default();
            warn!("market: pricing disabled because its saved targets could not be read: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fnv_sim::FormId;

    #[test]
    fn targets_roundtrip_without_enabling_optional_prices_or_storing_money() {
        let mut world = World::new();
        let mut state = Markets::default();
        state.book.quote(FormId(0x120), [(FormId(0x100), 12.0)]);
        world.insert_resource(state);
        let value = save(&world).unwrap();
        assert!(value.get("enabled").is_none());
        assert!(value.get("inventory").is_none());
        assert!(value.get("caps").is_none());
        let expected = world.resource::<Markets>().book.clone();
        world.insert_resource(Markets::default());
        load(&mut world, &value);
        assert_eq!(world.resource::<Markets>().book, expected);
        assert!(!world.resource::<Markets>().enabled);
    }

    #[test]
    fn missing_old_save_hook_keeps_default_vanilla_and_writes_no_extra() {
        let mut world = World::new();
        world.insert_resource(Markets::default());
        assert!(save(&world).is_none());
        assert!(!world.resource::<Markets>().enabled);
    }

    #[test]
    fn unsupported_saved_format_disables_pricing_without_a_world_transaction() {
        let mut world = World::new();
        world.insert_resource(Markets { enabled: true, book: MarketBook::default() });
        load(&mut world, &serde_json::json!({"format":"unknown", "book":{}}));
        assert!(!world.resource::<Markets>().enabled);
        assert!(save(&world).is_none());
    }
}
