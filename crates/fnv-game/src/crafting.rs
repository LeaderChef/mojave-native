//! Crafting: the recipes (RCPE) and their categories (RCCT), the recipe menu scripts open
//! (`ShowRecipeMenu`, alias `Craft`) and what crafting takes and gives
//! (docs/spec-notes/crafting.md).
//!
//! A recipe [documented: GECK wiki Recipe Entry, owner export]: a name; a category (where
//! it is made: Workbench, Campfire, Reloading Bench, …) and a subcategory (Aid, Ammo,
//! Chems, …, for sorting); a required skill and level ("NONE" for none); visibility
//! conditions (a recipe whose conditions fail is not listed at all); ingredients and
//! outputs, each an item and a quantity. "Recipes will only become usable once the
//! player has all the listed ingredients."
//!
//! The record's layout [documented: xEdit wbDefinitionsFNV RCPE: `EDID`, `FULL`, `CTDA`s,
//! `DATA` (skill as an actor value, i32, −1 none; level u32; category FormID; subcategory
//! FormID), then `RCIL` (an ingredient) and `RCOD` (an output), each followed by its
//! `RCQY` quantity, u32; RCCT: `EDID`, `FULL`, `DATA` flags u8, bit 0 Subcategory;
//! observed in FalloutNV.esm: 105 RCPE, every DATA 16 bytes, 263 RCIL + 144 RCOD = 407
//! RCQY; 10 RCCT, every DATA one byte].
//!
//! `ShowRecipeMenu category` takes a category or a subcategory [documented: GECK wiki
//! ShowRecipeMenu, its example `EDE1REF.Craft AmmoSubRecipes`]: the menu lists the
//! recipes whose category or subcategory it is. The menu is MenuMode 1077 [documented:
//! GECK wiki GetMenuTargetRef: "1077 The actor whose skills are used for crafting"].

use crate::cond::Condition;
use fnv_sim::FormId;
use std::collections::BTreeMap;

/// MenuMode of the recipe menu [documented: GECK wiki GetMenuTargetRef].
pub const RECIPE_MENU: u16 = 1077;

/// A recipe (RCPE).
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeDef {
    pub form: FormId,
    pub edid: Option<String>,
    pub name: String,
    /// Visibility conditions, run on the player.
    pub conditions: Vec<Condition>,
    /// The skill required (an actor value code) and its level; `None` for no skill.
    pub skill: Option<(u16, u32)>,
    pub category: Option<FormId>,
    pub subcategory: Option<FormId>,
    pub ingredients: Vec<(FormId, u32)>,
    pub outputs: Vec<(FormId, u32)>,
}

/// A checked batch, ready for the simulation's inventory exchange. Ingredients are
/// combined by item; output rows keep their authored order and independent leveled
/// list rolls. This contains quantities, not permission or a committed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeBatch {
    pub ingredients: Vec<(FormId, i32)>,
    pub outputs: Vec<(FormId, i32)>,
}

impl Default for RecipeDef {
    fn default() -> Self {
        RecipeDef { form: FormId(0), edid: None, name: String::new(), conditions: Vec::new(), skill: None, category: None, subcategory: None, ingredients: Vec::new(), outputs: Vec::new() }
    }
}

/// A recipe category (RCCT).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeCategoryDef {
    pub name: String,
    pub subcategory: bool,
}

impl RecipeDef {
    /// Read from the record's subrecords; `map` turns a local FormID into a load-order
    /// one, `cond` reads a CTDA. Ingredient and output quantities pair with the RCIL or
    /// RCOD before them.
    pub fn from_raw(form: FormId, subs: &[fnv_esm::Subrecord], map: &dyn Fn(u32) -> Option<u32>, cond: &mut dyn FnMut(&fnv_esm::Subrecord) -> Option<Condition>) -> RecipeDef {
        let mut r = RecipeDef { form, ..Default::default() };
        let u32_at = |b: &[u8], o: usize| b.get(o..o + 4).and_then(|x| <[u8; 4]>::try_from(x).ok()).map(u32::from_le_bytes);
        let fid = |v: Option<u32>| v.filter(|&x| x != 0).and_then(map).map(FormId);
        // The list the next RCQY belongs to.
        let mut last: Option<bool> = None;
        for s in subs {
            match &s.sig {
                b"EDID" => r.edid = Some(zstring(&s.data)),
                b"FULL" => r.name = zstring(&s.data),
                b"CTDA" => r.conditions.extend(cond(s)),
                b"DATA" => {
                    let skill = u32_at(&s.data, 0).map(|v| v as i32);
                    let level = u32_at(&s.data, 4).unwrap_or(0);
                    r.skill = skill.filter(|&k| k >= 0).map(|k| (k as u16, level));
                    r.category = fid(u32_at(&s.data, 8));
                    r.subcategory = fid(u32_at(&s.data, 12));
                }
                b"RCIL" | b"RCOD" => {
                    let input = &s.sig == b"RCIL";
                    if let Some(item) = fid(u32_at(&s.data, 0)) {
                        if input {
                            r.ingredients.push((item, 1));
                        } else {
                            r.outputs.push((item, 1));
                        }
                        last = Some(input);
                    } else {
                        last = None;
                    }
                }
                b"RCQY" => {
                    let n = u32_at(&s.data, 0).unwrap_or(1);
                    let list = match last.take() {
                        Some(true) => &mut r.ingredients,
                        Some(false) => &mut r.outputs,
                        None => continue,
                    };
                    if let Some(e) = list.last_mut() {
                        e.1 = n;
                    }
                }
                _ => {}
            }
        }
        r
    }

    /// Whether `category` (a category or a subcategory) lists this recipe.
    pub fn listed_under(&self, category: FormId) -> bool {
        self.category == Some(category) || self.subcategory == Some(category)
    }

    /// The total quantity made of the first output (what the menu shows after the name).
    pub fn made(&self) -> u32 {
        self.outputs.first().map_or(0, |o| o.1)
    }

    /// Check a positive number of makings without changing an inventory. Zero-count
    /// rows have no effect. Every combined input and output item total must fit i32;
    /// outputs remain separate rows so the caller can resolve each leveled list in order.
    pub fn batch(&self, times: u32) -> Option<RecipeBatch> {
        if times == 0 {
            return None;
        }
        let ingredients = aggregate_counts(&self.ingredients)?
            .into_iter()
            .map(|(item, count)| Some((item, batch_count(count, times)?)))
            .collect::<Option<Vec<_>>>()?;
        // Validate repeated output item totals without merging the actual output rows.
        for (_, count) in aggregate_counts(&self.outputs)? {
            batch_count(count, times)?;
        }
        let outputs = self.outputs.iter().filter(|(_, count)| *count > 0)
            .map(|&(item, count)| Some((item, batch_count(count, times)?)))
            .collect::<Option<Vec<_>>>()?;
        Some(RecipeBatch { ingredients, outputs })
    }
}

fn batch_count(count: u32, times: u32) -> Option<i32> {
    count.checked_mul(times).and_then(|n| i32::try_from(n).ok())
}

/// Positive counts, combined in first-appearance order. The inventory API accepts i32
/// counts, so saturating an oversized sum would silently change the authored recipe.
fn aggregate_counts(rows: &[(FormId, u32)]) -> Option<Vec<(FormId, u32)>> {
    let mut indices = BTreeMap::<FormId, usize>::new();
    let mut totals = Vec::<(FormId, u32)>::new();
    for &(item, count) in rows.iter().filter(|(_, count)| *count > 0) {
        i32::try_from(count).ok()?;
        if let Some(&i) = indices.get(&item) {
            let total = totals[i].1.checked_add(count)?;
            i32::try_from(total).ok()?;
            totals[i].1 = total;
        } else {
            indices.insert(item, totals.len());
            totals.push((item, count));
        }
    }
    Some(totals)
}

impl RecipeCategoryDef {
    pub fn from_raw(subs: &[fnv_esm::Subrecord]) -> RecipeCategoryDef {
        let mut c = RecipeCategoryDef::default();
        for s in subs {
            match &s.sig {
                b"FULL" => c.name = zstring(&s.data),
                b"DATA" => c.subcategory = s.data.first().is_some_and(|f| f & 1 != 0),
                _ => {}
            }
        }
        c
    }
}

fn zstring(b: &[u8]) -> String {
    // In this run's code page (fnv_util::text, #1439).
    fnv_util::text::decode_z(b).trim().to_string()
}

/// Why a recipe cannot be made now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// The skill is below the level: (skill, have, need).
    Skill(u16, u32, u32),
    /// Too few of an ingredient: (item, have, need).
    Ingredient(FormId, u32, u32),
    /// Zero makings or quantities that cannot fit a supported inventory exchange.
    InvalidBatch,
}

/// What making a recipe `times` times needs that the player lacks: the skill (`skill`
/// is the player's value of an actor value) and each ingredient (`have` counts the
/// player's items). Empty: skill and bounded batch requirements pass; the caller still
/// checks visibility and performs an atomic inventory exchange against its live state.
pub fn missing(r: &RecipeDef, times: u32, skill: impl Fn(u16) -> f64, have: impl Fn(FormId) -> i64) -> Vec<Missing> {
    let Some(batch) = r.batch(times) else { return vec![Missing::InvalidBatch] };
    let mut out = Vec::new();
    if let Some((av, level)) = r.skill {
        let v = skill(av).max(0.0).floor() as u32;
        if v < level {
            out.push(Missing::Skill(av, v, level));
        }
    }
    for (item, n) in batch.ingredients {
        let h = have(item).clamp(0, i64::from(u32::MAX)) as u32;
        let need = n as u32;
        if h < need {
            out.push(Missing::Ingredient(item, h, need));
        }
    }
    out
}

/// How many bounded batches the observed ingredients allow (skill and visibility aside).
/// Both input and output item totals must fit i32. With no positive rows the core has
/// no quantity bound and returns u32::MAX; the engine's no-input menu policy is separate.
/// Live destination capacity and resolved leveled outputs require the final exchange.
pub fn times_possible(r: &RecipeDef, have: impl Fn(FormId) -> i64) -> u32 {
    let Some(ingredients) = aggregate_counts(&r.ingredients) else { return 0 };
    let Some(outputs) = aggregate_counts(&r.outputs) else { return 0 };
    let limit = ingredients.iter().chain(&outputs).map(|(_, n)| i32::MAX as u32 / n).min().unwrap_or(u32::MAX);
    ingredients.into_iter().fold(limit, |limit, (item, need)| {
        let batches = have(item).max(0) as u64 / u64::from(need);
        limit.min(batches.min(u64::from(u32::MAX)) as u32)
    })
}

/// The skills' names by actor value code [documented: GECK wiki Actor Value Codes; the
/// AVIF records' names in FalloutNV.esm].
pub fn skill_name(av: u16) -> &'static str {
    use crate::actor_values::*;
    match av {
        BARTER => "Barter",
        BIG_GUNS => "Big Guns",
        ENERGY_WEAPONS => "Energy Weapons",
        EXPLOSIVES => "Explosives",
        LOCKPICK => "Lockpick",
        MEDICINE => "Medicine",
        MELEE_WEAPONS => "Melee Weapons",
        REPAIR => "Repair",
        SCIENCE => "Science",
        GUNS => "Guns",
        SNEAK => "Sneak",
        SPEECH => "Speech",
        SURVIVAL => "Survival",
        UNARMED => "Unarmed",
        _ => "Skill",
    }
}

/// `station.ShowRecipeMenu category`: the recipe menu, for the engine to draw; it is open
/// for scripts ([`RECIPE_MENU`]) until the engine closes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeRequest {
    /// The reference the menu was opened on (the workbench, the campfire).
    pub station: Option<FormId>,
    pub category: FormId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(sig: &[u8; 4], data: Vec<u8>) -> fnv_esm::Subrecord {
        fnv_esm::Subrecord { sig: *sig, data, extended: false }
    }

    fn word(v: u32) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }

    #[test]
    fn a_recipe_reads_its_data_and_pairs_quantities() {
        let mut data = word(40); // Science
        data.extend(word(45));
        data.extend(word(0x100));
        data.extend(word(0x200));
        let subs = vec![
            sub(b"EDID", b"RecipeX\0".to_vec()),
            sub(b"FULL", b"Conversion, X to Y (25)\0".to_vec()),
            sub(b"DATA", data),
            sub(b"RCIL", word(0x10)),
            sub(b"RCQY", word(5)),
            sub(b"RCIL", word(0x11)),
            sub(b"RCQY", word(1)),
            sub(b"RCOD", word(0x20)),
            sub(b"RCQY", word(25)),
        ];
        let r = RecipeDef::from_raw(FormId(7), &subs, &|l| Some(l), &mut |_| None);
        assert_eq!(r.name, "Conversion, X to Y (25)");
        assert_eq!(r.skill, Some((40, 45)));
        assert_eq!((r.category, r.subcategory), (Some(FormId(0x100)), Some(FormId(0x200))));
        assert_eq!(r.ingredients, vec![(FormId(0x10), 5), (FormId(0x11), 1)]);
        assert_eq!(r.outputs, vec![(FormId(0x20), 25)]);
        assert!(r.listed_under(FormId(0x100)) && r.listed_under(FormId(0x200)) && !r.listed_under(FormId(0x300)));
        // No skill: −1.
        let mut none = word(u32::MAX);
        none.extend([0u8; 12]);
        assert_eq!(RecipeDef::from_raw(FormId(8), &[sub(b"DATA", none)], &|l| Some(l), &mut |_| None).skill, None);
    }

    #[test]
    fn what_is_missing_and_how_often_it_can_be_made() {
        let r = RecipeDef { skill: Some((40, 45)), ingredients: vec![(FormId(1), 5), (FormId(2), 1)], outputs: vec![(FormId(3), 25)], ..Default::default() };
        let have = |n1: i64, n2: i64| move |i: FormId| if i == FormId(1) { n1 } else if i == FormId(2) { n2 } else { 0 };
        assert_eq!(missing(&r, 1, |_| 17.0, have(0, 1)), vec![Missing::Skill(40, 17, 45), Missing::Ingredient(FormId(1), 0, 5)]);
        assert!(missing(&r, 1, |_| 45.0, have(5, 1)).is_empty());
        assert_eq!(missing(&r, 2, |_| 50.0, have(9, 3)), vec![Missing::Ingredient(FormId(1), 9, 10)]);
        assert_eq!(times_possible(&r, have(12, 3)), 2);
        assert_eq!(times_possible(&r, have(12, 0)), 0);
        assert_eq!(r.made(), 25);
    }

    #[test]
    fn duplicate_ingredient_rows_require_their_combined_quantity() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 2), (FormId(1), 3)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert_eq!(missing(&r, 1, |_| 0.0, |_| 4), vec![Missing::Ingredient(FormId(1), 4, 5)]);
        assert_eq!(missing(&r, 2, |_| 0.0, |_| 9), vec![Missing::Ingredient(FormId(1), 9, 10)]);
    }

    #[test]
    fn duplicate_ingredient_rows_limit_the_number_of_whole_batches() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 2), (FormId(1), 3)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert_eq!(times_possible(&r, |_| 10), 2);
        assert_eq!(times_possible(&r, |_| 4), 0);
    }

    #[test]
    fn a_saturated_requirement_cannot_authorize_an_overflowing_batch() {
        let r = RecipeDef { ingredients: vec![(FormId(1), u32::MAX)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert!(!missing(&r, 2, |_| 0.0, |_| i64::from(u32::MAX)).is_empty());
        assert_eq!(times_possible(&r, |_| i64::MAX), 0);
    }

    #[test]
    fn individually_valid_duplicate_rows_cannot_overflow_the_item_total() {
        let r = RecipeDef { ingredients: vec![(FormId(1), i32::MAX as u32), (FormId(1), 1)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert!(!missing(&r, 1, |_| 0.0, |_| i64::MAX).is_empty());
        assert_eq!(times_possible(&r, |_| i64::MAX), 0);
    }

    #[test]
    fn an_output_that_cannot_fit_the_inventory_is_not_craftable() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 1)], outputs: vec![(FormId(2), i32::MAX as u32 + 1)], ..Default::default() };
        assert!(!missing(&r, 1, |_| 0.0, |_| 10).is_empty());
        assert_eq!(times_possible(&r, |_| 10), 0);
    }

    #[test]
    fn large_inventory_observations_do_not_wrap_to_zero() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 1)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        for have in [i64::from(u32::MAX) + 1, i64::MAX] {
            assert!(missing(&r, 1, |_| 0.0, |_| have).is_empty());
            assert_eq!(times_possible(&r, |_| have), i32::MAX as u32);
        }
    }

    #[test]
    fn zero_batches_are_rejected_rather_than_silently_treated_as_one() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 1)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert!(!missing(&r, 0, |_| 0.0, |_| 10).is_empty());
    }

    #[test]
    fn checked_batch_combines_inputs_but_preserves_each_output_row() {
        let r = RecipeDef {
            ingredients: vec![(FormId(2), 0), (FormId(3), 2), (FormId(1), 1), (FormId(3), 4)],
            outputs: vec![(FormId(8), 2), (FormId(7), 1), (FormId(8), 3), (FormId(9), 0)],
            ..Default::default()
        };
        assert_eq!(r.batch(3), Some(RecipeBatch { ingredients: vec![(FormId(3), 18), (FormId(1), 3)], outputs: vec![(FormId(8), 6), (FormId(7), 3), (FormId(8), 9)] }));
        assert_eq!(r.ingredients.len(), 4, "checking a batch must not rewrite the recipe");
        assert_eq!(r.outputs.len(), 4);
    }

    #[test]
    fn zero_count_rows_do_not_observe_or_require_an_inventory_item() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 0)], outputs: vec![(FormId(2), 0)], ..Default::default() };
        assert!(missing(&r, 1, |_| panic!("no skill required"), |_| panic!("zero rows require no stock")).is_empty());
        assert_eq!(times_possible(&r, |_| panic!("zero rows require no stock")), u32::MAX);
        assert_eq!(r.batch(1), Some(RecipeBatch { ingredients: Vec::new(), outputs: Vec::new() }));
        assert_eq!(r.batch(0), None);
    }

    #[test]
    fn repeated_output_rows_keep_independent_rolls_but_share_the_item_count_bound() {
        let r = RecipeDef { outputs: vec![(FormId(8), i32::MAX as u32), (FormId(7), 1), (FormId(8), 1)], ..Default::default() };
        assert_eq!(r.batch(1), None);
        assert_eq!(missing(&r, 1, |_| 0.0, |_| 0), vec![Missing::InvalidBatch]);
        assert_eq!(times_possible(&r, |_| 0), 0);
    }

    #[test]
    fn checked_batch_multiplication_never_wraps_or_saturates() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 2)], outputs: vec![(FormId(2), 1)], ..Default::default() };
        assert_eq!(r.batch(u32::MAX), None);
        assert_eq!(missing(&r, u32::MAX, |_| 0.0, |_| i64::MAX), vec![Missing::InvalidBatch]);
        assert_eq!(times_possible(&r, |_| i64::MAX), i32::MAX as u32 / 2);
    }

    #[test]
    fn output_capacity_bounds_the_batch_even_with_abundant_ingredients() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 1)], outputs: vec![(FormId(2), 3)], ..Default::default() };
        let limit = i32::MAX as u32 / 3;
        assert_eq!(times_possible(&r, |_| i64::MAX), limit);
        assert_eq!(r.batch(limit).unwrap().outputs, vec![(FormId(2), (limit * 3) as i32)]);
        assert_eq!(r.batch(limit + 1), None);
    }

    #[test]
    fn duplicate_ingredients_observe_each_unique_item_once() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 2), (FormId(1), 3), (FormId(2), 1)], ..Default::default() };
        let calls = std::cell::Cell::new(0);
        let have = |_: FormId| { calls.set(calls.get() + 1); 10 };
        assert!(missing(&r, 1, |_| 0.0, have).is_empty());
        assert_eq!(calls.get(), 2);
        calls.set(0);
        assert_eq!(times_possible(&r, have), 2);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn availability_and_missing_agree_on_combined_whole_batches() {
        let r = RecipeDef { ingredients: vec![(FormId(1), 2), (FormId(1), 3)], outputs: vec![(FormId(2), 2)], ..Default::default() };
        for stock in -2..=40 {
            let possible = times_possible(&r, |_| stock);
            assert_eq!(possible, (stock.max(0) / 5) as u32);
            for times in 1..=10 {
                assert_eq!(missing(&r, times, |_| 0.0, |_| stock).is_empty(), times <= possible, "stock {stock}, batch {times}");
            }
        }
    }

    #[test]
    fn an_empty_recipe_has_no_core_quantity_bound_but_zero_batch_is_invalid() {
        let r = RecipeDef::default();
        assert_eq!(times_possible(&r, |_| panic!("no ingredients")), u32::MAX);
        assert!(missing(&r, 1, |_| panic!("no skill"), |_| panic!("no ingredients")).is_empty());
        assert_eq!(missing(&r, 0, |_| 0.0, |_| 0), vec![Missing::InvalidBatch]);
    }

    #[test]
    fn a_category_knows_whether_it_is_a_subcategory() {
        assert_eq!(RecipeCategoryDef::from_raw(&[sub(b"FULL", b"Workbench\0".to_vec()), sub(b"DATA", vec![0])]), RecipeCategoryDef { name: "Workbench".into(), subcategory: false });
        assert!(RecipeCategoryDef::from_raw(&[sub(b"DATA", vec![1])]).subcategory);
    }
}
