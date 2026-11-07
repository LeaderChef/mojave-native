//! [design] An all-or-nothing inventory exchange for recipe production. The original
//! `add_item` and `remove_item` script semantics stay unchanged. No second stock book.

use super::*;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InventoryExchangeError {
    #[error(transparent)]
    Sim(#[from] SimError),
    #[error("item quantity exceeds inventory capacity: {0}")]
    CountOverflow(FormId),
    #[error("not enough {item}: have {have}, need {need}")]
    Missing { item: FormId, have: i32, need: i32 },
}

impl World<'_> {
    /// [design] Consume all inputs and deliver all resolved outputs, or change neither
    /// the inventory nor its RNG. A first-use reference is also rolled back on error.
    /// Inputs are aggregated; output rows retain order and independent leveled rolls.
    /// ChanceNone remains valid; unsafe resolver truncation, arithmetic saturation
    /// or recursive work exhaustion refuses the entire exchange.
    /// Returned quantities are actual resolved products, not requested list counts.
    /// Equipment/ownership on unconsumed groups is retained. No script events, skill
    /// checks, stat grants or station authorisation are implied by this core primitive.
    pub fn exchange_inventory(
        &mut self,
        owner: FormId,
        consume: &[(FormId, i32)],
        produce: &[(FormId, i32)],
    ) -> Result<Vec<(FormId, i32)>, InventoryExchangeError> {
        let mut needs = BTreeMap::<FormId, i32>::new();
        for &(item, count) in consume {
            if count <= 0 {
                return Err(SimError::BadCount(count).into());
            }
            self.check_item(item)?;
            let total = needs.entry(item).or_default();
            *total = total.checked_add(count).ok_or(InventoryExchangeError::CountOverflow(item))?;
        }
        for &(item, count) in produce {
            if count <= 0 {
                return Err(SimError::BadCount(count).into());
            }
            if self.forms.leveled(item).is_none() {
                self.check_item(item)?;
            }
        }

        // initial() may roll carried/death items and materialise a reference. Preserve
        // exactly those owners on failure rather than cloning the entire world.
        let existed = self.refs.contains_key(&owner);
        let old_rng = self.rng.clone();
        let old_serial = self.presence_serial;
        let old_death = self.death_items.get(&owner).copied();
        let result = (|| {
            let mut inventory = self.inventory(owner)?.clone();
            for (item, need) in needs {
                let have = inventory.count(item);
                if have < need {
                    return Err(InventoryExchangeError::Missing { item, have, need });
                }
                let removed = inventory.remove(item, need);
                if removed != need {
                    return Err(InventoryExchangeError::Missing { item, have: removed, need });
                }
            }

            let mut rng = self.rng.clone();
            let mut added = Vec::new();
            for &(item, count) in produce {
                let mut resolved = Vec::new();
                leveled::resolve_checked_with(self.forms, &self.globals, &self.leveled_added,
                    &self.leveled_edits, item, count, self.cfg.player_level, &mut rng,
                    &mut resolved).map_err(SimError::from)?;
                for entry in resolved.into_iter().filter(|e| e.count > 0) {
                    self.check_item(entry.item)?;
                    let before = inventory.count(entry.item);
                    let total = before.checked_add(entry.count)
                        .filter(|&n| n >= 0)
                        .ok_or(InventoryExchangeError::CountOverflow(entry.item))?;
                    if inventory.add_extra(entry.item, entry.count, entry.extra) != total {
                        return Err(InventoryExchangeError::CountOverflow(entry.item));
                    }
                    added.push((entry.item, entry.count));
                }
            }
            // No callbacks or fallible operation between the two commits.
            self.refs.get_mut(&owner).expect("inventory materialised above").inventory = Some(inventory);
            self.rng = rng;
            Ok(added)
        })();
        if result.is_err() {
            self.rng = old_rng;
            if !existed {
                self.refs.remove(&owner);
                self.presence_serial = old_serial;
                match old_death {
                    Some(item) => { self.death_items.insert(owner, item); }
                    None => { self.death_items.remove(&owner); }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests;
