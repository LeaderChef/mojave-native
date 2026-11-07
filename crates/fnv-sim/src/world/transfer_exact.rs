//! [design] Exact, atomic transfers between the world's real inventories. No
//! parallel stock ledger and no change to the ordinary up-to-count transfer API.

use super::*;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InventoryTransferError {
    #[error(transparent)]
    Sim(#[from] SimError),
    #[error("source and destination are the same inventory: {0}")]
    SameHolder(FormId),
    #[error("item quantity exceeds inventory capacity: {0}")]
    CountOverflow(FormId),
    #[error("not enough {item}: have {have}, need {need}")]
    Missing { item: FormId, have: i32, need: i32 },
}

// Avoid Inventory::count's unchecked sum when checking an already oversized
// stack assembled by an ordinary API. This path never silently saturates a total.
fn checked_count(inventory: &Inventory, item: FormId) -> Result<i32, InventoryTransferError> {
    inventory.groups().filter(|group| group.0 == item).try_fold(0_i32, |sum, (_, _, count)| {
        sum.checked_add(count).filter(|&total| total >= 0)
            .ok_or(InventoryTransferError::CountOverflow(item))
    })
}

impl World<'_> {
    /// [design] Move every requested unit or change nothing, including first-use
    /// reference/death-inventory materialization, the RNG and presence serial.
    /// Positive duplicate rows are checked and aggregated; success returns actual
    /// moved totals in FormID order. Lists are not resolved as newly created stock.
    /// Condition, owner and mod flags travel with each actual group. Moved units
    /// become unequipped, just as with `transfer_at`; untransferred groups retain
    /// their equipment state. This does not imply events, payment or authorisation.
    /// Both holders are validated/materialized even for an empty successful batch.
    pub fn transfer_inventory_exact(
        &mut self,
        from: FormId,
        to: FormId,
        items: &[(FormId, i32)],
    ) -> Result<Vec<(FormId, i32)>, InventoryTransferError> {
        if from == to {
            return Err(InventoryTransferError::SameHolder(from));
        }
        let mut needs = BTreeMap::<FormId, i32>::new();
        for &(item, count) in items {
            if count <= 0 {
                return Err(SimError::BadCount(count).into());
            }
            self.check_item(item)?;
            let total = needs.entry(item).or_default();
            *total = total.checked_add(count).ok_or(InventoryTransferError::CountOverflow(item))?;
        }
        let rows: Vec<_> = needs.into_iter().map(|(item, count)| (item, count, Pick::ANY)).collect();
        self.transfer_inventory_exact_at(from, to, &rows)
    }

    /// [design] Exact selected-row batch. Rows consume the candidate source in
    /// caller order, so overlapping picks cannot reuse units taken by an earlier
    /// row. A short later pick refuses the entire batch. Returned actual totals
    /// aggregate by FormID, while the moved units preserve their own extra data
    /// (except equipped, cleared as in `transfer_at`). Picks use `Extra::picked`
    /// directly; no sorting, equality aggregation or float/NaN key is introduced.
    /// Validation/materialization and rollback guarantees match the ANY wrapper.
    pub fn transfer_inventory_exact_at(
        &mut self,
        from: FormId,
        to: FormId,
        items: &[(FormId, i32, Pick)],
    ) -> Result<Vec<(FormId, i32)>, InventoryTransferError> {
        if from == to {
            return Err(InventoryTransferError::SameHolder(from));
        }
        // Only request totals are aggregated. Distinct/overlapping picks retain
        // their order, and cannot evade an overflowing aggregate request.
        let mut needs = BTreeMap::<FormId, i32>::new();
        for &(item, count, _) in items {
            if count <= 0 {
                return Err(SimError::BadCount(count).into());
            }
            self.check_item(item)?;
            let total = needs.entry(item).or_default();
            *total = total.checked_add(count).ok_or(InventoryTransferError::CountOverflow(item))?;
        }

        let existed = [self.refs.contains_key(&from), self.refs.contains_key(&to)];
        let old_death = [self.death_items.get(&from).copied(), self.death_items.get(&to).copied()];
        let old_rng = self.rng.clone();
        let old_presence = self.presence_serial;
        let result = (|| {
            // Match ordinary transfer's destination-first materialization order.
            // Only these two inventories are copied, never the whole World.
            let mut destination = self.inventory(to)?.clone();
            let mut source = self.inventory(from)?.clone();
            let mut moved = BTreeMap::<FormId, i32>::new();
            for &(item, need, pick) in items {
                checked_count(&source, item)?;
                let mut total = checked_count(&destination, item)?;
                total.checked_add(need).ok_or(InventoryTransferError::CountOverflow(item))?;
                let groups = unequipped(source.take_at(item, need, pick));
                let actual = groups.iter().try_fold(0_i32, |sum, (_, count)| {
                    sum.checked_add(*count).ok_or(InventoryTransferError::CountOverflow(item))
                })?;
                if actual != need {
                    return Err(InventoryTransferError::Missing { item, have: actual, need });
                }
                for (extra, count) in groups {
                    total = total.checked_add(count).ok_or(InventoryTransferError::CountOverflow(item))?;
                    if destination.add_extra(item, count, extra) != total {
                        return Err(InventoryTransferError::CountOverflow(item));
                    }
                }
                let transferred = moved.entry(item).or_default();
                *transferred = transferred.checked_add(actual).ok_or(InventoryTransferError::CountOverflow(item))?;
            }
            // Both holders exist. No callback, RNG roll or fallible operation lies
            // between the commits; failures above have only edited local candidates.
            self.refs.get_mut(&from).expect("source materialized above").inventory = Some(source);
            self.refs.get_mut(&to).expect("destination materialized above").inventory = Some(destination);
            Ok(moved.into_iter().collect())
        })();
        if result.is_err() {
            self.rng = old_rng;
            self.presence_serial = old_presence;
            for ((owner, was_present), death) in [from, to].into_iter().zip(existed).zip(old_death) {
                if !was_present {
                    self.refs.remove(&owner);
                }
                match death {
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
