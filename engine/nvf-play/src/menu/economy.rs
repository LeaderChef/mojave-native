//! Optional stock-pressure pricing, as plain data; no inventory or caps authority.
//!
//! [design] Each vendor stock container remembers one target per relevant item: its
//! first observed vendor quality-equivalent quantity, at least eight. Targets persist;
//! actual quantities come from the caller's current world inventory. Include the
//! player/vendor item union
//! and an explicit zero for items the vendor has none of. Repeated item entries are
//! vendor-stack quality weights, summed before the first target is captured. The caller
//! weights each stack by `count * condition_value / base_value` for positive, finite
//! values, and zero otherwise. The ratio has no upper clamp: configured values above
//! one remain proportional. Ordinary valued consumables contribute one per item,
//! fully broken equipment zero. This does not
//! exclude weapons/armour or let cheap broken units change pressure like sound equipment.
//! Negative/nonfinite observation weights are zero; finite sums saturate only at the
//! representable f64 maximum. These defensive rules authorize no inventory transaction.
//!
//! [design] At stock `s` and target `t`, marginal pressure is
//! `clamp(1.5 - s / (2t), 0.5, 1.5)`. A cart uses the analytic mean over its whole stock
//! interval, not its initial unit price multiplied by quantity. Player buying lowers
//! vendor stock; selling raises it. The same interval costs the same in either direction
//! before existing barter discounts and rounding, so scarcity alone cannot profit from
//! buying a large cart and immediately selling it back. This is an engine option, not a
//! claim about original-game behaviour or a finite simulated world economy. Vanilla
//! restocking remains outside this module and unchanged.
//!
//! Only targets are serializable here. Quotes are immutable snapshots, not transactions:
//! the caller still checks live stock, player inventory, funds and cart validity, then
//! applies the existing trade. Unit-row previews use that row's one-item quality weight;
//! cart lines use aggregated quality weight across its condition stacks. No NPC stock,
//! money, clock, restock or settlement is created.

use fnv_sim::FormId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MIN_TARGET: f64 = 8.0;

/// Persistent price anchors, by vendor stock container and item. No balances or current
/// inventories are stored. Validate a deserialized book before enabling its prices.
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketBook {
    #[serde(default)]
    targets: BTreeMap<FormId, BTreeMap<FormId, f64>>,
}

impl MarketBook {
    /// Reject invalid persisted anchors; no invented stock, replacement target or broad
    /// structural limit is applied. The loading integration decides how to disable an
    /// invalid optional book. Targets must be finite and strictly positive.
    pub fn validate(&self) -> Result<(), String> {
        for (stock, items) in &self.targets {
            for (item, target) in items {
                if !target.is_finite() || *target <= 0.0 {
                    return Err(format!("market target must be finite and positive for stock {stock}, item {item}"));
                }
            }
        }
        Ok(())
    }

    /// Freeze targets on the first observation of each item and snapshot current vendor
    /// quality weights. The iterator must describe vendor stock, never the player's quantities.
    /// Missing entries are unknown, not an assertion of zero; pass explicit zeros for
    /// relevant player items absent from stock. Duplicate vendor stacks are combined
    /// before capturing their target, independent of iterator order.
    pub fn quote(&mut self, stock: FormId, quantities: impl IntoIterator<Item = (FormId, f64)>) -> MarketQuote {
        let mut counts: BTreeMap<FormId, f64> = BTreeMap::new();
        for (item, quantity) in quantities {
            let count = counts.entry(item).or_default();
            *count = add_weight(*count, weight(quantity));
        }
        let targets = self.targets.entry(stock).or_default();
        let items = counts
            .into_iter()
            .map(|(item, current)| {
                let target = *targets.entry(item).or_insert(current.max(MIN_TARGET));
                (item, QuoteItem { current, target })
            })
            .collect();
        MarketQuote { stock, items }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct QuoteItem {
    current: f64,
    target: f64,
}

/// An immutable pricing view for one container; making later quotes cannot change it.
/// It is deliberately not saved: live world inventories provide every new current count.
#[derive(Clone, Debug, PartialEq)]
pub struct MarketQuote {
    stock: FormId,
    items: BTreeMap<FormId, QuoteItem>,
}

impl MarketQuote {
    pub fn stock(&self) -> FormId {
        self.stock
    }

    pub fn current(&self, item: FormId) -> Option<f64> {
        self.items.get(&item).map(|q| q.current)
    }

    pub fn target(&self, item: FormId) -> Option<f64> {
        self.items.get(&item).map(|q| q.target)
    }

    /// A fresh immutable snapshot of the same known items and frozen targets, from
    /// actual vendor counts. Omitted known items are now zero. Unknown items are
    /// ignored rather than capturing a target mid-session: prime the relevant union
    /// through MarketBook::quote when opening the trade. The original quote is unchanged.
    pub fn rebase_stock(&self, quantities: impl IntoIterator<Item = (FormId, f64)>) -> MarketQuote {
        let mut next = self.clone();
        for item in next.items.values_mut() {
            item.current = 0.0;
        }
        for (item, quantity) in quantities {
            if let Some(known) = next.items.get_mut(&item) {
                known.current = add_weight(known.current, weight(quantity));
            }
        }
        next
    }

    /// Mean pressure over the cart's vendor-stock interval, in constant work regardless
    /// of quality-equivalent quantity. `buying` means the player buys from this vendor. Buying is clamped
    /// to available stock for pricing; this never authorizes an unavailable quantity.
    /// Zero units (or buying from empty stock) return the current marginal pressure.
    /// An item omitted from the snapshot, or an invalid target used despite failed
    /// validation, has neutral pressure. Negative/nonfinite requested weights are zero.
    /// Callers can detect omission with the getters.
    pub fn average_multiplier(&self, item: FormId, buying: bool, units: f64) -> f32 {
        let Some(q) = self.items.get(&item).filter(|q| q.target.is_finite() && q.target > 0.0) else { return 1.0 };
        let current = q.current;
        let target = q.target;
        let units = if buying { weight(units).min(current) } else { weight(units) };
        if units == 0.0 {
            return pressure(current, target) as f32;
        }
        let low = if buying { (current - units).max(0.0) } else { current };
        interval_mean(low, units, target).clamp(0.5, 1.5) as f32
    }
}

/// Inputs are non-authoritative observations; malformed weights cannot poison quotes.
fn weight(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 { value } else { 0.0 }
}

fn add_weight(a: f64, b: f64) -> f64 {
    (a + b).min(f64::MAX)
}

fn pressure(stock: f64, target: f64) -> f64 {
    (1.5 - (stock / target) * 0.5).clamp(0.5, 1.5)
}

/// [design] Analytic integral divided by interval width: below 2t, one trapezoid;
/// after it, constant 0.5. Stock/width normalized by target avoid overflowing 2t,
/// low+width or the area itself, and avoid subtracting large antiderivatives. Neither
/// the work nor the number of pieces depends on how many physical items were traded.
fn interval_mean(low: f64, units: f64, target: f64) -> f64 {
    let start = low / target;
    if start >= 2.0 {
        return 0.5;
    }
    let width = units / target;
    if width == 0.0 {
        return pressure(low, target);
    }
    let ramp_left = 2.0 - start;
    if width <= ramp_left {
        1.5 - 0.5 * start - 0.25 * width
    } else {
        // The excess over the plateau is a triangle occupying ramp_left/width of
        // the interval. An infinite ratio means that finite excess is negligible.
        0.5 + 0.5 * (1.0 - 0.5 * start) * (ramp_left / width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STOCK: FormId = FormId(0x100);
    const OTHER_STOCK: FormId = FormId(0x101);
    const ITEM: FormId = FormId(0x200);
    const OTHER_ITEM: FormId = FormId(0x201);

    fn at(current: f64, target: f64) -> MarketQuote {
        let mut book = MarketBook::default();
        book.quote(STOCK, [(ITEM, target)]);
        book.quote(STOCK, [(ITEM, current)])
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!((actual - expected).abs() <= tolerance, "{actual} != {expected} (tolerance {tolerance})");
    }

    #[test]
    fn marginal_pressure_has_scarce_target_and_abundant_endpoints() {
        for (stock, expected) in [(0.0, 1.5), (8.0, 1.0), (16.0, 0.5), (32.0, 0.5)] {
            let quote = at(stock, 8.0);
            assert_eq!(quote.average_multiplier(ITEM, true, 0.0), expected);
            assert_eq!(quote.average_multiplier(ITEM, false, 0.0), expected);
        }
    }

    #[test]
    fn one_unit_means_match_the_linear_marginal_endpoints() {
        let quote = at(8.0, 8.0);
        // Buy [7,8]: (1.0625+1)/2. Sell [8,9]: (1+0.9375)/2.
        assert_eq!(quote.average_multiplier(ITEM, true, 1.0), 1.03125);
        assert_eq!(quote.average_multiplier(ITEM, false, 1.0), 0.96875);
        assert_eq!(at(0.0, 8.0).average_multiplier(ITEM, false, 1.0), 1.46875);
    }

    #[test]
    fn cart_uses_the_whole_interval_not_the_first_unit_preview() {
        let quote = at(8.0, 8.0);
        assert_eq!(quote.average_multiplier(ITEM, true, 8.0), 1.25);
        assert_eq!(quote.average_multiplier(ITEM, false, 8.0), 0.75);
        assert_ne!(quote.average_multiplier(ITEM, true, 8.0), quote.average_multiplier(ITEM, true, 1.0));
        // Cross the knee at 16: [12,32] and [12,20].
        close(f64::from(at(32.0, 8.0).average_multiplier(ITEM, true, 20.0)), 0.525, 1e-7);
        assert_eq!(at(12.0, 8.0).average_multiplier(ITEM, false, 8.0), 0.5625);
    }

    #[test]
    fn successive_split_stacks_have_additive_buy_and_sell_integrals() {
        // Both directions traverse [4,24], across the linear/constant boundary.
        let whole_buy = f64::from(at(24.0, 8.0).average_multiplier(ITEM, true, 20.0)) * 20.0;
        let split_buy = f64::from(at(24.0, 8.0).average_multiplier(ITEM, true, 7.0)) * 7.0
            + f64::from(at(17.0, 8.0).average_multiplier(ITEM, true, 13.0)) * 13.0;
        let whole_sell = f64::from(at(4.0, 8.0).average_multiplier(ITEM, false, 20.0)) * 20.0;
        let split_sell = f64::from(at(4.0, 8.0).average_multiplier(ITEM, false, 7.0)) * 7.0
            + f64::from(at(11.0, 8.0).average_multiplier(ITEM, false, 13.0)) * 13.0;
        close(whole_buy, 14.5, 2e-6);
        close(split_buy, whole_buy, 2e-6);
        close(split_sell, whole_sell, 2e-6);
        assert_eq!(whole_buy, whole_sell);
    }

    #[test]
    fn buying_then_selling_back_uses_the_identical_interval() {
        for stock in [1.0, 8.0, 16.0, 32.0, f64::from(u32::MAX)] {
            for requested in [1.0_f64, 7.0, 16.0, f64::from(u32::MAX)] {
                let units = requested.min(stock);
                let paid = at(stock, 8.0).average_multiplier(ITEM, true, units);
                let received = at(stock - units, 8.0).average_multiplier(ITEM, false, units);
                assert_eq!(paid.to_bits(), received.to_bits(), "same pressure interval for stock {stock}, units {units}");
            }
        }
    }

    #[test]
    fn duplicate_vendor_stacks_combine_before_the_target_is_captured() {
        let mut split = MarketBook::default();
        let mut whole = MarketBook::default();
        let a = split.quote(STOCK, [(ITEM, 2.0), (OTHER_ITEM, 0.0), (ITEM, 18.0)]);
        let b = whole.quote(STOCK, [(ITEM, 20.0), (OTHER_ITEM, 0.0)]);
        assert_eq!(a, b);
        assert_eq!(split, whole);
        assert_eq!(a.current(ITEM), Some(20.0));
        assert_eq!(a.target(ITEM), Some(20.0));
        assert_eq!(a.current(OTHER_ITEM), Some(0.0));
        assert_eq!(a.target(OTHER_ITEM), Some(8.0));
    }

    #[test]
    fn oversized_buys_and_empty_stock_never_cross_negative_stock() {
        let quote = at(3.0, 8.0);
        assert_eq!(quote.average_multiplier(ITEM, true, f64::MAX), quote.average_multiplier(ITEM, true, 3.0));
        assert_eq!(quote.average_multiplier(ITEM, true, 3.0), 1.40625);
        assert_eq!(at(0.0, 8.0).average_multiplier(ITEM, true, f64::MAX), 1.5);
    }

    #[test]
    fn extreme_counts_stay_finite_bounded_and_do_not_wrap() {
        for target in [8.0, f64::from(u32::MAX), f64::MAX] {
            for current in [0.0, 1.0, 8.0, f64::from(u32::MAX), f64::MAX] {
                for units in [0.0, 1.0, f64::from(u32::MAX), f64::MAX] {
                    for buying in [false, true] {
                        let price = at(current, target).average_multiplier(ITEM, buying, units);
                        assert!(price.is_finite() && (0.5..=1.5).contains(&price));
                    }
                }
            }
        }
        let quote = at(f64::from(u32::MAX), f64::from(u32::MAX));
        close(f64::from(quote.average_multiplier(ITEM, true, 1.0)), 1.0, 1e-7);
        let mut book = MarketBook::default();
        let quote = book.quote(STOCK, [(ITEM, f64::from(u32::MAX)), (ITEM, 1.0)]);
        assert_eq!(quote.current(ITEM), Some(f64::from(u32::MAX) + 1.0));
        assert_eq!(quote.target(ITEM), quote.current(ITEM), "no old u32 saturation remains");
        let saturated = book.quote(OTHER_STOCK, [(ITEM, f64::MAX), (ITEM, f64::MAX)]);
        assert_eq!(saturated.current(ITEM), Some(f64::MAX));
        assert_eq!(saturated.target(ITEM), Some(f64::MAX));
        assert_eq!(saturated.average_multiplier(ITEM, true, f64::MAX), 1.25);
        assert_eq!(saturated.average_multiplier(ITEM, false, f64::MAX), 0.75);
        assert!(book.validate().is_ok());
    }

    #[test]
    fn initial_targets_are_frozen_per_container_and_item() {
        let mut book = MarketBook::default();
        book.quote(STOCK, [(ITEM, 2.0)]);
        assert_eq!(book.quote(STOCK, [(ITEM, 80.0)]).target(ITEM), Some(8.0));
        assert_eq!(book.quote(OTHER_STOCK, [(ITEM, 80.0)]).target(ITEM), Some(80.0));
        assert_eq!(book.quote(STOCK, [(OTHER_ITEM, 70.0)]).target(OTHER_ITEM), Some(70.0));
        book.quote(STOCK, []);
        assert_eq!(book.quote(STOCK, [(ITEM, 0.0)]).target(ITEM), Some(8.0), "absence or depletion never resets a target");
        assert!(book.validate().is_ok());
    }

    #[test]
    fn a_later_quote_cannot_mutate_an_existing_snapshot() {
        let mut book = MarketBook::default();
        let first = book.quote(STOCK, [(ITEM, 8.0)]);
        let second = book.quote(STOCK, [(ITEM, 2.0)]);
        assert_eq!(first.stock(), STOCK);
        assert_eq!(first.current(ITEM), Some(8.0));
        assert_eq!(second.current(ITEM), Some(2.0));
        assert_eq!(first.average_multiplier(ITEM, true, 8.0), 1.25);
        assert_eq!(first.target(ITEM), second.target(ITEM));
        assert_eq!(first.current(OTHER_ITEM), None);
        assert_eq!(first.target(OTHER_ITEM), None);
        assert_eq!(first.average_multiplier(OTHER_ITEM, true, 1.0), 1.0);
    }

    #[test]
    fn rebasing_replaces_actual_counts_but_never_captures_or_changes_targets() {
        let mut book = MarketBook::default();
        let first = book.quote(STOCK, [(ITEM, 20.0), (OTHER_ITEM, 0.0)]);
        let unchanged_book = book.clone();
        let unknown = FormId(0x202);
        let refreshed = first.rebase_stock([(ITEM, 2.0), (unknown, 99.0), (ITEM, 3.0)]);
        assert_eq!(refreshed.stock(), STOCK);
        assert_eq!(refreshed.current(ITEM), Some(5.0));
        assert_eq!(refreshed.target(ITEM), Some(20.0));
        assert_eq!(refreshed.current(OTHER_ITEM), Some(0.0));
        assert_eq!(refreshed.target(OTHER_ITEM), Some(8.0));
        assert_eq!(refreshed.current(unknown), None);
        assert_eq!(refreshed.target(unknown), None);
        assert_eq!(first.current(ITEM), Some(20.0), "the old snapshot remains immutable");
        assert_eq!(book, unchanged_book, "rebasing has no persistent-book write");
        let empty = refreshed.rebase_stock([]);
        assert_eq!(empty.current(ITEM), Some(0.0));
        assert_eq!(empty.current(OTHER_ITEM), Some(0.0));
        assert_eq!(empty.target(ITEM), Some(20.0));
        assert_eq!(empty.average_multiplier(ITEM, true, f64::MAX), 1.5);
        assert_eq!(empty.average_multiplier(OTHER_ITEM, false, 1.0), 1.46875);
        let max = refreshed.rebase_stock([(ITEM, f64::MAX), (ITEM, f64::MAX)]);
        assert_eq!(max.current(ITEM), Some(f64::MAX));
        assert_eq!(max.target(ITEM), Some(20.0));
    }

    #[test]
    fn serde_roundtrip_keeps_only_targets_and_preserves_their_history() {
        let mut book = MarketBook::default();
        book.quote(STOCK, [(ITEM, 8.25), (OTHER_ITEM, 0.0)]);
        book.quote(OTHER_STOCK, [(ITEM, 99.0)]);
        book.quote(STOCK, [(ITEM, 123.0)]);
        let saved = serde_json::to_value(&book).unwrap();
        assert_eq!(saved, serde_json::json!({
            "targets": {
                "0x00000100": {"0x00000200": 8.25, "0x00000201": 8.0},
                "0x00000101": {"0x00000200": 99.0}
            }
        }));
        let mut restored: MarketBook = serde_json::from_value(saved).unwrap();
        assert_eq!(restored, book);
        assert!(restored.validate().is_ok());
        let quote = restored.quote(STOCK, [(ITEM, 1000.0), (OTHER_ITEM, 3.0)]);
        assert_eq!(quote.current(ITEM), Some(1000.0), "current counts come from the new world observation");
        assert_eq!(quote.target(ITEM), Some(8.25));
        assert_eq!(quote.target(OTHER_ITEM), Some(8.0));
    }

    #[test]
    fn invalid_zero_targets_are_rejected_without_replenishment_or_repair() {
        let mut invalid: MarketBook = serde_json::from_value(serde_json::json!({
            "targets": {"0x00000100": {"0x00000200": 0.0}}
        })).unwrap();
        assert!(invalid.validate().is_err());
        let quote = invalid.quote(STOCK, [(ITEM, 100.0)]);
        assert_eq!(quote.target(ITEM), Some(0.0), "quote does not silently repair corrupt persisted data");
        assert_eq!(quote.average_multiplier(ITEM, true, f64::MAX), 1.0);
        assert!(invalid.validate().is_err());
        let empty: MarketBook = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, MarketBook::default());
        assert!(empty.validate().is_ok());
    }

    #[test]
    fn fractional_quality_intervals_and_stack_weights_are_preserved() {
        let quote = at(8.25, 8.0);
        assert_eq!(quote.current(ITEM), Some(8.25));
        assert_eq!(quote.average_multiplier(ITEM, true, 0.25), 0.9921875);
        assert_eq!(quote.average_multiplier(ITEM, false, 0.25), 0.9765625);
        let reverse = at(8.0, 8.0).average_multiplier(ITEM, false, 0.25);
        assert_eq!(quote.average_multiplier(ITEM, true, 0.25).to_bits(), reverse.to_bits());
        let mut book = MarketBook::default();
        let combined = book.quote(STOCK, [(ITEM, 2.5), (ITEM, 5.75)]);
        assert_eq!(combined.current(ITEM), Some(8.25));
        assert_eq!(combined.target(ITEM), Some(8.25));
        // Caller-supplied quality above one remains proportional, not capped at count.
        assert_eq!(combined.rebase_stock([(ITEM, 12.5)]).current(ITEM), Some(12.5));
    }

    #[test]
    fn zero_quality_units_do_not_change_stock_targets_or_pressure() {
        let mut book = MarketBook::default();
        let sound = book.quote(STOCK, [(ITEM, 8.0)]);
        let with_broken = book.quote(STOCK, [(ITEM, 8.0), (ITEM, 0.0), (ITEM, 0.0)]);
        assert_eq!(sound, with_broken);
        assert_eq!(sound.rebase_stock([(ITEM, 8.0), (ITEM, 0.0)]), sound);
        assert_eq!(with_broken.average_multiplier(ITEM, true, 1.0), sound.average_multiplier(ITEM, true, 1.0));
        assert_eq!(with_broken.average_multiplier(ITEM, false, 0.0), 1.0, "zero quality has zero interval width");
        let only_broken = book.quote(OTHER_STOCK, [(ITEM, 0.0), (ITEM, 0.0)]);
        assert_eq!(only_broken.current(ITEM), Some(0.0));
        assert_eq!(only_broken.target(ITEM), Some(8.0));
        assert_eq!(only_broken.average_multiplier(ITEM, false, 0.0), 1.5);
        assert_eq!(only_broken.current(ITEM), Some(0.0), "pricing creates no stock");
    }

    #[test]
    fn malformed_observation_and_cart_weights_are_zero_without_poisoning() {
        let mut book = MarketBook::default();
        let quote = book.quote(STOCK, [(ITEM, -1.0), (ITEM, f64::NAN), (ITEM, f64::INFINITY), (ITEM, f64::NEG_INFINITY)]);
        assert_eq!(quote.current(ITEM), Some(0.0));
        assert_eq!(quote.target(ITEM), Some(8.0));
        assert!(book.validate().is_ok());
        let refreshed = quote.rebase_stock([(ITEM, 8.0), (ITEM, -3.0), (ITEM, f64::NAN), (ITEM, f64::INFINITY)]);
        assert_eq!(refreshed.current(ITEM), Some(8.0));
        assert_eq!(refreshed.target(ITEM), Some(8.0));
        for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(refreshed.average_multiplier(ITEM, true, invalid), 1.0);
            assert_eq!(refreshed.average_multiplier(ITEM, false, invalid), 1.0);
        }
        assert!(book.validate().is_ok());
    }

    #[test]
    fn nonfinite_and_negative_persisted_targets_fail_validation_without_repair() {
        // JSON cannot encode nonfinite floats. Construct these corrupt in-memory values
        // explicitly, so validation also protects non-JSON producers and future loaders.
        for target in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            let mut book = MarketBook {
                targets: BTreeMap::from([(STOCK, BTreeMap::from([(ITEM, target)]))]),
            };
            assert!(book.validate().is_err());
            let quote = book.quote(STOCK, [(ITEM, 8.0)]);
            assert_eq!(quote.target(ITEM).unwrap().to_bits(), target.to_bits());
            assert_eq!(quote.average_multiplier(ITEM, true, 1.0), 1.0);
            assert_eq!(quote.average_multiplier(ITEM, false, 1.0), 1.0);
            assert!(book.validate().is_err());
        }
    }
}
