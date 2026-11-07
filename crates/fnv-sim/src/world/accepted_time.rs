//! [design] Monotonic accepted work time, separate from the editable calendar.
//! Calendar precision and merchant-restock policy stay in their existing owners.

use super::*;

pub(super) fn saved_zero() -> Option<String> {
    Some("0".into())
}

pub(super) fn restore_hours(saved: Option<&str>) -> Result<f64, String> {
    saved.and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| "invalid accepted_game_hours: expected finite nonnegative decimal text".into())
}

pub(super) fn validate_reservations(forms: &FormStore, snapshot: &Snapshot) -> Result<(), String> {
    for (&id, &token) in &snapshot.inventory_reservations {
        let valid = token != 0 && id.is_dynamic() && !snapshot.deleted_created.contains_key(&id)
            && snapshot.refs.get(&id).is_some_and(|state| {
            !state.deleted() && state.inventory.is_some() && state.extra.owner == Some(FormId::PLAYER_BASE)
                && forms.get(state.base).is_some_and(|base| {
                    base.script.is_none() && matches!(&base.data, BaseData::Container(container) if !container.respawns)
                })
            });
        if !valid {
            return Err(format!("invalid inventory reservation for {id}: expected a nonzero token and an owned, live, nonrespawning, scriptless dynamic container"));
        }
    }
    Ok(())
}

impl World<'_> {
    /// [design] Successful positive intervals accepted by the calendar store. This
    /// is not a date or wall clock: direct global writes and UI time do not grant
    /// work. None permanently invalidates work credit after numeric exhaustion.
    pub fn accepted_game_hours(&self) -> Option<f64> {
        self.accepted_game_hours.filter(|hours| hours.is_finite() && *hours >= 0.0)
    }

    /// [design] The transaction token reserving this real inventory, if any.
    pub fn inventory_reservation(&self, id: FormId) -> Option<u64> {
        self.inventory_reservations.get(&id).copied()
    }

    // Called only after store_calendar succeeds. advance_real_seconds reaches
    // this through advance_hours, so there is exactly one credit per interval.
    pub(super) fn accept_game_hours(&mut self, hours: f64) {
        if !hours.is_finite() || hours <= 0.0 {
            return;
        }
        self.accepted_game_hours = self.accepted_game_hours().and_then(|before| {
            let after = before + hours;
            (after.is_finite() && after > before).then_some(after)
        });
    }
}

#[cfg(test)]
mod tests;
