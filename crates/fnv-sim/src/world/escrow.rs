//! [design] Dedicated, reserved real inventories for production orders. These helpers
//! do not grant station access, worker consent, recipe eligibility or job completion.

use super::*;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EscrowError {
    #[error(transparent)]
    Sim(#[from] SimError),
    #[error(transparent)]
    Transfer(#[from] InventoryTransferError),
    #[error(transparent)]
    Exchange(#[from] InventoryExchangeError),
    #[error("a production reservation token must be nonzero")]
    InvalidToken,
    #[error("production reservation token {0} is already active")]
    TokenInUse(u64),
    #[error("{0} is not a reserved production inventory")]
    NotReserved(FormId),
    #[error("reservation token does not match production inventory {0}")]
    WrongToken(FormId),
    #[error("{0} is not an empty, non-respawning, script-free container base")]
    InvalidStorageBase(FormId),
    #[error("production inventory {0} is missing or its storage identity changed")]
    InvalidEscrow(FormId),
    #[error("dynamic reference ids are exhausted")]
    DynamicIdsExhausted,
    #[error("dynamic reference id {0} is already in use")]
    DynamicIdCollision(FormId),
}

impl World<'_> {
    /// Read-only validation of the same token, live storage and full base/session
    /// invariant used at commit. Save admission must not accept weaker storage.
    pub fn validate_production_escrow(&self, escrow: FormId, token: u64) -> Result<(), EscrowError> {
        self.production_token(escrow, token)
    }

    fn production_storage_base(&self, base: FormId) -> Result<(), EscrowError> {
        let Some(form) = self.forms.get(base) else { return Err(SimError::UnknownForm(base).into()) };
        let BaseData::Container(container) = &form.data else { return Err(EscrowError::InvalidStorageBase(base)) };
        if form.sig != *b"CONT" || form.script.is_some() || container.respawns
            || !container.entries.is_empty() || self.base_edits.get(&base).is_some_and(|entries| !entries.is_empty())
        {
            return Err(EscrowError::InvalidStorageBase(base));
        }
        Ok(())
    }

    fn production_token(&self, escrow: FormId, token: u64) -> Result<(), EscrowError> {
        if token == 0 {
            return Err(EscrowError::InvalidToken);
        }
        match self.inventory_reservations.get(&escrow) {
            None => return Err(EscrowError::NotReserved(escrow)),
            Some(&current) if current != token => return Err(EscrowError::WrongToken(escrow)),
            Some(_) => {}
        }
        let Some(state) = self.refs.get(&escrow) else { return Err(EscrowError::InvalidEscrow(escrow)) };
        if !escrow.is_dynamic() || state.deleted() || state.enabled
            || state.extra.owner != Some(FormId::PLAYER_BASE) || state.inventory.is_none()
        {
            return Err(EscrowError::InvalidEscrow(escrow));
        }
        self.production_storage_base(state.base).map_err(|_| EscrowError::InvalidEscrow(escrow))
    }

    /// [design] Fund a new, disabled real container and reserve it in the same
    /// exclusive World call. The anchor is read without materialising it. A failed
    /// deposit leaves no new reference/id, source mutation, death roll or RNG change.
    /// Only the touched source state is backed up; the whole World is never cloned.
    /// The returned id is allocated dynamically, never inferred from a template id.
    pub fn create_production_escrow(
        &mut self,
        from: FormId,
        storage_base: FormId,
        anchor: FormId,
        items: &[(FormId, i32)],
        token: u64,
    ) -> Result<FormId, EscrowError> {
        if token == 0 {
            return Err(EscrowError::InvalidToken);
        }
        if self.inventory_reservations.values().any(|&current| current == token) {
            return Err(EscrowError::TokenInUse(token));
        }
        self.production_storage_base(storage_base)?;
        let (cell, pos, rot) = if let Some(state) = self.refs.get(&anchor) {
            if state.deleted() { return Err(SimError::Deleted(anchor).into()) }
            (state.cell, state.pos, state.rot)
        } else if let Some(placed) = self.placed.get(anchor) {
            if placed.flags & 0x20 != 0 { return Err(SimError::Deleted(anchor).into()) }
            (placed.cell, placed.pos, placed.rot)
        } else if anchor == FormId::PLAYER_REF {
            (None, [0.0; 3], [0.0; 3])
        } else {
            return Err(SimError::UnknownRef(anchor).into());
        };
        let id = FormId(self.next_dynamic);
        let next = self.next_dynamic.checked_add(1).filter(|_| id.is_dynamic())
            .ok_or(EscrowError::DynamicIdsExhausted)?;
        if self.refs.contains_key(&id) || self.deleted_created.contains_key(&id)
            || self.inventory_reservations.contains_key(&id)
        {
            return Err(EscrowError::DynamicIdCollision(id));
        }
        if from == id {
            return Err(InventoryTransferError::SameHolder(id).into());
        }
        let old_source = self.refs.get(&from).cloned();
        let old_death = self.death_items.get(&from).copied();
        let old_rng = self.rng.clone();
        let old_presence = self.presence_serial;
        let old_next = self.next_dynamic;
        // The validated base has no initial stock or script. Build exactly that
        // empty container, without using a normal visible PlaceAtMe as an escrow.
        self.refs.insert(id, RefState {
            base: storage_base, cell, pos, rot, enabled: false, deleted: false,
            count: 1, inventory: Some(Inventory::new()), lock: None, open: false,
            extra: Extra { owner: Some(FormId::PLAYER_BASE), ..Extra::default() },
            actor_values: BTreeMap::new(), last_restock: None, scale: None,
            equip_locks: BTreeMap::new(), dropped_by: None, outfitted: false,
        });
        self.next_dynamic = next;
        self.presence_serial += 1;
        match self.transfer_inventory_exact(from, id, items) {
            Ok(_) => {
                self.inventory_reservations.insert(id, token);
                Ok(id)
            }
            Err(error) => {
                self.refs.remove(&id);
                match old_source {
                    Some(state) => { self.refs.insert(from, state); }
                    None => { self.refs.remove(&from); }
                }
                match old_death {
                    Some(item) => { self.death_items.insert(from, item); }
                    None => { self.death_items.remove(&from); }
                }
                self.rng = old_rng;
                self.presence_serial = old_presence;
                self.next_dynamic = old_next;
                Err(error.into())
            }
        }
    }

    /// [design] Resolve an atomic production exchange only under its exact live
    /// token. Normal inventory callers remain blocked outside this non-reentrant
    /// call. A failed exchange retains the same real input stock and reservation.
    pub fn exchange_production_escrow(
        &mut self,
        escrow: FormId,
        token: u64,
        consume: &[(FormId, i32)],
        produce: &[(FormId, i32)],
    ) -> Result<Vec<(FormId, i32)>, EscrowError> {
        self.production_token(escrow, token)?;
        self.inventory_reservations.remove(&escrow);
        let result = self.exchange_inventory(escrow, consume, produce);
        self.inventory_reservations.insert(escrow, token);
        result.map_err(EscrowError::from)
    }

    /// [design] Deliver all actual remaining groups (inputs on cancellation, outputs
    /// on collection), then close this storage. A capacity or destination failure
    /// changes neither inventory and reinstates its reservation. No original inputs
    /// are recreated after production. Closed-token history is the job ledger's job;
    /// this layer rejects duplicate release because the inventory is no longer reserved.
    pub fn release_production_escrow(
        &mut self,
        escrow: FormId,
        token: u64,
        to: FormId,
    ) -> Result<Vec<(FormId, i32)>, EscrowError> {
        self.production_token(escrow, token)?;
        if to == escrow {
            return Err(InventoryTransferError::SameHolder(escrow).into());
        }
        let inventory = self.refs.get(&escrow).and_then(|s| s.inventory.as_ref()).expect("validated above");
        let mut totals = BTreeMap::<FormId, i32>::new();
        for (item, _, count) in inventory.groups() {
            let n = totals.entry(item).or_default();
            *n = n.checked_add(count).filter(|&n| n >= 0)
                .ok_or(InventoryTransferError::CountOverflow(item))?;
        }
        let items: Vec<_> = totals.into_iter().collect();
        self.inventory_reservations.remove(&escrow);
        match self.transfer_inventory_exact(escrow, to, &items) {
            Ok(moved) => {
                // Transfer has committed. The source is a known created, empty,
                // disabled ref, so deletion contains no fallible lookup/callback.
                self.refs.get_mut(&escrow).expect("validated escrow survives transfer").deleted = true;
                self.presence_serial += 1;
                Ok(moved)
            }
            Err(error) => {
                self.inventory_reservations.insert(escrow, token);
                Err(error.into())
            }
        }
    }
}

#[cfg(test)]
mod tests;
