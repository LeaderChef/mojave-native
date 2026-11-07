//! [design] Persistent host-authoritative work orders. Materials live in a reserved
//! real World inventory, never in this ledger. Normal recipes use accepted game time;
//! long-running work may instead use a persisted host UTC deadline. No NPC, network
//! server, building model or elapsed-time acceleration is implied by this module.

use crate::{Game, GameData};
use crate::crafting::{RecipeBatch, RecipeDef};
use fnv_sim::{FormId, Inventory, World};
use fnv_sim::forms::BaseData;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const BOOK_FORMAT: &str = "j6.production/1";
pub const GAME_HOURS_PER_BATCH: f64 = 0.25;
pub const TWO_DAYS_SECONDS: u64 = 48 * 60 * 60;
const MAX_HISTORY: usize = 4096;
const MAX_ACTIVE: usize = 64;
const REACH: f32 = 512.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "clock", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkDeadline {
    // Decimal strings keep the exact finite f64 through every save reader.
    GameHours { started: String, due: String },
    HostUtc { started: u64, due: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkTiming {
    GameHours(f64),
    /// Trusted host sample and a real duration in seconds, never a client's clock.
    HostUtc { now: u64, seconds: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderState { Working, Ready, Collected, Cancelled }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionOrder {
    pub id: u64,
    pub station: FormId,
    pub station_base: FormId,
    pub cell: FormId,
    pub recipe: FormId,
    pub category: FormId,
    pub times: u32,
    /// Full public RecipeDef debug descriptor includes CTDA params (its serde view
    /// intentionally omits params). Conservative identity check, not a signature.
    pub recipe_binding: String,
    pub inputs: Vec<(FormId, i32)>,
    pub outputs: Vec<(FormId, i32)>,
    pub escrow: FormId,
    pub escrow_base: FormId,
    pub deadline: WorkDeadline,
    pub state: OrderState,
    /// Read-only proof of real stock, including condition/mod/owner groups.
    pub seal: Inventory,
}

impl ProductionOrder {
    fn active(&self) -> bool { matches!(self.state, OrderState::Working | OrderState::Ready) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionBook {
    pub schema: String,
    pub next_id: u64,
    pub revision: u64,
    pub orders: BTreeMap<u64, ProductionOrder>,
    pub last_host_utc: Option<u64>,
    // Rebuilt against the actual restored World. Never spendable or persisted.
    #[serde(skip)]
    next_game_due: Option<f64>,
    #[serde(skip)]
    next_host_due: Option<u64>,
    /// Failed due orders are held until a reload or explicit operator repair, rather
    /// than cloning stock and logging errors every render frame.
    #[serde(skip)]
    held: BTreeMap<u64, String>,
}

impl PartialEq for ProductionBook {
    fn eq(&self, other: &Self) -> bool {
        self.schema == other.schema && self.next_id == other.next_id
            && self.revision == other.revision && self.orders == other.orders
            && self.last_host_utc == other.last_host_utc
    }
}

impl Default for ProductionBook {
    fn default() -> Self {
        Self { schema: BOOK_FORMAT.into(), next_id: 1, revision: 0,
            orders: BTreeMap::new(), last_host_utc: None,
            next_game_due: None, next_host_due: None, held: BTreeMap::new() }
    }
}

pub struct StationOrderSummary {
    pub id: u64,
    pub text: String,
    pub collect: bool,
    pub cancel: bool,
}

fn finite(text: &str) -> Result<f64, String> {
    text.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0)
        .ok_or_else(|| "invalid production game-time deadline".into())
}

fn totals(rows: &[(FormId, i32)]) -> Result<BTreeMap<FormId, i32>, String> {
    let mut out = BTreeMap::<FormId, i32>::new();
    for &(item, count) in rows {
        if count <= 0 { return Err("invalid production quantity".into()) }
        let n = out.entry(item).or_default();
        *n = n.checked_add(count).ok_or("production quantity overflow")?;
    }
    if out.is_empty() { return Err("empty production batch".into()) }
    Ok(out)
}

fn stock_totals(stock: &Inventory) -> Result<BTreeMap<FormId, i32>, String> {
    if stock.groups().any(|(_, extra, _)| extra.condition.is_some_and(|c| !c.is_finite())) {
        return Err("production stock has nonfinite item condition metadata".into());
    }
    totals(&stock.groups().map(|(i, _, n)| (i, n)).collect::<Vec<_>>())
}

fn ordinary_batch(world: &World<'_>, recipe: &RecipeDef, times: u32) -> Result<RecipeBatch, String> {
    let batch = recipe.batch(times).ok_or("invalid production batch")?;
    totals(&batch.ingredients)?;
    totals(&batch.outputs)?;
    for &(item, _) in batch.ingredients.iter().chain(&batch.outputs) {
        if !world.forms.get(item).is_some_and(|f| matches!(f.data, BaseData::Item(_))) {
            return Err(format!("production requires a concrete item: {item}"));
        }
    }
    Ok(batch)
}

fn pose(world: &World<'_>, id: FormId) -> Result<(FormId, FormId, [f32; 3]), String> {
    if let Some(r) = world.peek(id) {
        if r.deleted() { return Err(format!("production reference {id} was deleted")) }
        Ok((r.base, r.cell.ok_or("production station has no cell")?, r.pos))
    } else if let Some(r) = world.placed.get(id) {
        if r.flags & 0x20 != 0 { return Err(format!("production reference {id} was deleted")) }
        Ok((r.base, r.cell.ok_or("production station has no cell")?, r.pos))
    } else { Err(format!("unknown production reference {id}")) }
}

fn station(world: &World<'_>, id: FormId) -> Result<(FormId, FormId, [f32; 3]), String> {
    if id == FormId::PLAYER_REF || !world.is_enabled(id).map_err(|e| e.to_string())? {
        return Err("use a nearby enabled crafting station".into());
    }
    let p = pose(world, id)?;
    if !world.forms.get(p.0).is_some_and(|f| f.sig == *b"ACTI") {
        return Err("production needs an actual crafting activator".into());
    }
    Ok(p)
}

fn station_recipe(data: &GameData, world: &World<'_>, base: FormId, recipe: &RecipeDef) -> Result<(), String> {
    // First supported stations are the two audited vanilla activators. This is a
    // new opt-in rule, not permission to run every recipe at every ACTI reference.
    let edid = world.forms.get(base).and_then(|f| f.edid.as_deref()).unwrap_or_default();
    let category = if edid.eq_ignore_ascii_case("WorkBench") { "WorkbenchRecipes" }
        else if edid.eq_ignore_ascii_case("CampfireCrafting01") { "CampfireRecipes" }
        else { return Err("queued work is not supported at this station yet".into()) };
    // RCCT belongs to the game-rule registry, not the simulation base-form index.
    if data.edids.get(&category.to_ascii_lowercase()).is_none_or(|id| recipe.category != Some(*id)) {
        return Err("this recipe belongs at a different crafting station".into());
    }
    Ok(())
}

fn near_player(world: &World<'_>, at: FormId) -> Result<(FormId, FormId), String> {
    let (base, cell, pos) = station(world, at)?;
    let player = world.peek(FormId::PLAYER_REF).ok_or("player position is unavailable")?;
    let distance: f32 = player.pos.iter().zip(pos).map(|(a, b)| (*a - b).powi(2)).sum();
    if player.deleted() || player.cell != Some(cell) || !distance.is_finite() || distance > REACH * REACH {
        return Err("move closer to this crafting station".into());
    }
    Ok((base, cell))
}

impl ProductionBook {
    pub fn is_pristine(&self) -> bool {
        self.schema == BOOK_FORMAT && self.next_id == 1 && self.revision == 0
            && self.orders.is_empty() && self.last_host_utc.is_none()
    }

    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema != BOOK_FORMAT || self.next_id != self.orders.len() as u64 + 1 || self.orders.len() > MAX_HISTORY {
            return Err("unsupported or oversized production ledger".into());
        }
        let mut escrows = BTreeSet::new();
        let mut stations = BTreeSet::new();
        for (offset, (&key, o)) in self.orders.iter().enumerate() {
            if key != offset as u64 + 1 || o.id != key || key >= self.next_id || o.times == 0
                || !o.escrow.is_dynamic() || o.recipe_binding.is_empty() || o.recipe_binding.len() > 65536
            { return Err("invalid production order identity".into()) }
            if !escrows.insert(o.escrow) { return Err("production escrow is reused".into()) }
            totals(&o.inputs)?;
            totals(&o.outputs)?;
            match &o.deadline {
                WorkDeadline::GameHours { started, due } => {
                    if finite(due)? <= finite(started)? { return Err("invalid production duration".into()) }
                }
                WorkDeadline::HostUtc { started, due } => {
                    if due <= started || self.last_host_utc.is_none_or(|t| t < *started) {
                        return Err("invalid host production deadline".into());
                    }
                }
            }
            if o.active() {
                if !stations.insert(o.station) { return Err("station has conflicting work orders".into()) }
                let expected = if o.state == OrderState::Working { &o.inputs } else { &o.outputs };
                if stock_totals(&o.seal)? != totals(expected)? { return Err("production stock proof disagrees with its batch".into()) }
            }
        }
        if stations.len() > MAX_ACTIVE { return Err("too many active work orders".into()) }
        if self.revision < self.orders.len() as u64 { return Err("production history has no revision".into()) }
        Ok(())
    }

    fn check_binding(o: &ProductionOrder, data: &GameData, world: &World<'_>) -> Result<(), String> {
        let recipe = data.recipes.get(&o.recipe).ok_or("production recipe is missing")?;
        let batch = ordinary_batch(world, recipe, o.times)?;
        if format!("{recipe:?}") != o.recipe_binding || !recipe.listed_under(o.category)
            || batch.ingredients != o.inputs || batch.outputs != o.outputs
        { return Err("production recipe changed; restore the matching plugins".into()) }
        let (base, cell, _) = station(world, o.station)?;
        if base != o.station_base || cell != o.cell { return Err("production station identity changed".into()) }
        station_recipe(data, world, base, recipe)?;
        Self::check_stock(o, world)
    }

    fn check_stock(o: &ProductionOrder, world: &World<'_>) -> Result<(), String> {
        world.validate_production_escrow(o.escrow, o.id).map_err(|e| e.to_string())?;
        let r = world.peek(o.escrow).ok_or("production storage is missing")?;
        if world.inventory_reservation(o.escrow) != Some(o.id) || r.deleted() || r.enabled()
            || r.base != o.escrow_base || r.cell != Some(o.cell)
            || r.extra.owner != Some(FormId::PLAYER_BASE) || r.inventory.as_ref() != Some(&o.seal)
        { return Err("production storage or its stock changed; no transaction committed".into()) }
        Ok(())
    }

    /// Fail closed against the actual restored World, including orphan reservations.
    pub fn validate_world(&mut self, data: &GameData, world: &World<'_>) -> Result<(), String> {
        self.validate_shape()?;
        for o in self.orders.values().filter(|o| o.active()) {
            Self::check_binding(o, data, world)?;
            if let WorkDeadline::GameHours { started, .. } = &o.deadline {
                if world.accepted_game_hours().is_none_or(|h| h < finite(started).unwrap_or(f64::INFINITY)) {
                    return Err("production game clock is unavailable or moved backwards".into());
                }
            }
        }
        for (id, _) in world.changed() {
            if let Some(token) = world.inventory_reservation(id) {
                if !self.orders.get(&token).is_some_and(|o| o.active() && o.escrow == id) {
                    return Err("reserved production stock has no matching order".into());
                }
            }
        }
        self.schedule();
        Ok(())
    }

    fn schedule(&mut self) {
        self.next_game_due = None;
        self.next_host_due = None;
        for o in self.orders.values().filter(|o| o.state == OrderState::Working && !self.held.contains_key(&o.id)) {
            match &o.deadline {
                WorkDeadline::GameHours { due, .. } => {
                    if let Ok(d) = finite(due) { self.next_game_due = Some(self.next_game_due.map_or(d, |x| x.min(d))) }
                }
                WorkDeadline::HostUtc { due, .. } => self.next_host_due = Some(self.next_host_due.map_or(*due, |x| x.min(*due))),
            }
        }
    }

    pub fn has_host_orders(&self) -> bool { self.next_host_due.is_some() }

    pub fn station_summary(&self, station: FormId, game_hours: Option<f64>) -> Option<StationOrderSummary> {
        let o = self.orders.values().find(|o| o.station == station && o.active())?;
        let text = if let Some(why) = self.held.get(&o.id) { format!("Order #{} held: {why}", o.id) }
        else if o.state == OrderState::Ready { format!("Order #{} ready to collect", o.id) } else {
            match &o.deadline {
                WorkDeadline::GameHours { due, .. } => {
                    let minutes = game_hours.zip(finite(due).ok()).map(|(h,d)| ((d-h).max(0.0)*60.0).ceil() as u64);
                    minutes.map_or_else(|| "Work paused: game clock unavailable".into(), |m| format!("Order #{}: {m} game minutes remaining", o.id))
                }
                WorkDeadline::HostUtc { due, .. } => {
                    let seconds = due.saturating_sub(self.last_host_utc.unwrap_or(0));
                    format!("Order #{}: {}h {}m real time remaining (continues offline)", o.id, seconds/3600, seconds%3600/60)
                }
            }
        };
        Some(StationOrderSummary { id: o.id, text, collect: o.state == OrderState::Ready, cancel: o.state == OrderState::Working })
    }
}

impl Game<'_> {
    fn production_access(&self, at: FormId) -> Result<(FormId, FormId), String> {
        let binding = near_player(&self.world, at)?;
        if self.owned_by_other(at).is_some() || self.trespass_cell().is_some()
            || self.world.lock_of(at).is_some_and(|lock| lock.locked)
        { return Err("you do not have access to this crafting station".into()) }
        Ok(binding)
    }

    pub fn queue_recipe_order(&mut self, station: FormId, category: FormId, recipe: FormId, times: u32) -> Result<u64, String> {
        self.queue_production_order(station, category, recipe, times, self.production.next_id,
            WorkTiming::GameHours(GAME_HOURS_PER_BATCH * f64::from(times)))
    }

    /// Trusted-host entry point. A future server must admit requests here itself;
    /// clients may not select elapsed time, materials, output or completion state.
    pub fn queue_production_order(&mut self, at: FormId, category: FormId, recipe_id: FormId,
        times: u32, id: u64, timing: WorkTiming) -> Result<u64, String>
    {
        self.production.validate_shape()?;
        if let Some(old) = self.production.orders.get(&id) {
            let same_time = match (&old.deadline, timing) {
                (WorkDeadline::GameHours { started, due }, WorkTiming::GameHours(h)) => (finite(due)?-finite(started)?-h).abs() <= 1e-9,
                (WorkDeadline::HostUtc { started, due }, WorkTiming::HostUtc { seconds, .. }) => due-started == seconds,
                _ => false,
            };
            return if old.station == at && old.category == category && old.recipe == recipe_id && old.times == times && same_time {
                Ok(id)
            } else { Err("production command id was already used for different work".into()) };
        }
        if id != self.production.next_id || self.production.orders.len() >= MAX_HISTORY
            || self.production.orders.values().filter(|o| o.active()).count() >= MAX_ACTIVE
        { return Err("production command id or ledger capacity is invalid".into()) }
        if self.production.orders.values().any(|o| o.station == at && o.active()) { return Err("this station already has a work order".into()) }
        let next_id = id.checked_add(1).ok_or("production ids exhausted")?;
        let revision = self.production.revision.checked_add(1).ok_or("production revisions exhausted")?;
        let (base, cell) = self.production_access(at)?;
        let recipe = self.data.recipes.get(&recipe_id).ok_or("unknown production recipe")?.clone();
        station_recipe(self.data, &self.world, base, &recipe)?;
        if !recipe.listed_under(category) || !self.conditions_pass(&recipe.conditions, Some(FormId::PLAYER_REF), None, "production admission") {
            return Err("recipe requirements changed".into());
        }
        if let Some((av, level)) = recipe.skill {
            if crate::functions::actor_value_capped(self, FormId::PLAYER_REF, av) < f64::from(level) {
                return Err("required crafting skill is too low".into());
            }
        }
        let batch = ordinary_batch(&self.world, &recipe, times)?;
        let carried = self.world.peek(FormId::PLAYER_REF).and_then(|r| r.inventory.as_ref()).ok_or("player inventory is unavailable")?;
        if carried.groups().any(|(item, extra, _)| batch.ingredients.iter().any(|(input,_)| *input == item)
            && extra.condition.is_some_and(|c| !c.is_finite()))
        { return Err("production input has nonfinite item condition metadata".into()) }
        let deadline = match timing {
            WorkTiming::GameHours(hours) => {
                let start = self.world.accepted_game_hours().ok_or("game work clock unavailable")?;
                let due = start + hours;
                if !hours.is_finite() || hours <= 0.0 || !due.is_finite() || due <= start { return Err("invalid game-time work duration".into()) }
                WorkDeadline::GameHours { started: start.to_string(), due: due.to_string() }
            }
            WorkTiming::HostUtc { now, seconds } => {
                let start = now.max(self.production.last_host_utc.unwrap_or(0));
                let due = start.checked_add(seconds).filter(|d| *d > start).ok_or("invalid real-time work duration")?;
                WorkDeadline::HostUtc { started: start, due }
            }
        };
        let storage = self.world.forms.by_edid("MetalBox01Empty").ok_or("empty production container base is unavailable")?;
        let escrow = self.world.create_production_escrow(FormId::PLAYER_REF, storage, at, &batch.ingredients, id).map_err(|e| e.to_string())?;
        // No fallible operations after the World transaction commits.
        let seal = self.world.peek(escrow).and_then(|r| r.inventory.clone()).expect("created escrow inventory");
        if let WorkDeadline::HostUtc { started, .. } = &deadline { self.production.last_host_utc = Some(*started) }
        self.production.orders.insert(id, ProductionOrder { id, station: at, station_base: base, cell,
            recipe: recipe_id, category, times, recipe_binding: format!("{recipe:?}"),
            inputs: batch.ingredients, outputs: batch.outputs, escrow, escrow_base: storage,
            deadline, state: OrderState::Working, seal });
        self.production.next_id = next_id;
        self.production.revision = revision;
        self.production.schedule();
        Ok(id)
    }

    /// Host UTC and accepted game time are separate domains. Repeated samples, cold
    /// restarts after a deadline and backwards host samples cannot complete twice.
    pub fn advance_production_orders(&mut self, host_utc: Option<u64>) -> Vec<String> {
        if self.production.orders.is_empty() { return vec![] }
        if self.production.has_host_orders() {
            if let Some(now) = host_utc { self.production.last_host_utc = Some(now.max(self.production.last_host_utc.unwrap_or(0))) }
        }
        let hours = self.world.accepted_game_hours();
        let game_due = self.production.next_game_due.zip(hours).is_some_and(|(d,h)| h >= d);
        let host_due = host_utc.is_some() && self.production.next_host_due.zip(self.production.last_host_utc).is_some_and(|(d,h)| h >= d);
        if !game_due && !host_due { return vec![] }
        let ids: Vec<_> = self.production.orders.values().filter(|o| o.state == OrderState::Working && !self.production.held.contains_key(&o.id)).filter(|o| match &o.deadline {
            WorkDeadline::GameHours { due, .. } => hours.zip(finite(due).ok()).is_some_and(|(h,d)| h >= d),
            WorkDeadline::HostUtc { due, .. } => host_utc.is_some() && self.production.last_host_utc.is_some_and(|h| h >= *due),
        }).map(|o| o.id).collect();
        let mut notices = Vec::new();
        for id in ids {
            let o = &self.production.orders[&id];
            let Ok(revision) = self.production.revision.checked_add(1).ok_or(()) else { break };
            if let Err(e) = ProductionBook::check_binding(o, self.data, &self.world) {
                notices.push(format!("Order #{id} held: {e}"));
                self.production.held.insert(id, e); continue;
            }
            match self.world.exchange_production_escrow(o.escrow, id, &o.inputs, &o.outputs) {
                Err(e) => {
                    notices.push(format!("Order #{id} held: {e}"));
                    self.production.held.insert(id, e.to_string());
                }
                Ok(_) => {
                    let o = self.production.orders.get_mut(&id).expect("existing order");
                    o.seal = self.world.peek(o.escrow).and_then(|r| r.inventory.clone()).expect("completed escrow");
                    o.state = OrderState::Ready;
                    self.production.revision = revision;
                    let (times, inputs) = (o.times, o.inputs.iter().map(|(i,_)| *i).collect::<Vec<_>>());
                    self.count_stat(crate::misc_stats::ITEMS_CRAFTED, f64::from(times));
                    self.challenge_crafted(&inputs, times);
                    notices.push(format!("Order #{id} ready to collect"));
                }
            }
        }
        self.production.schedule();
        notices
    }

    pub fn collect_production_order(&mut self, id: u64) -> Result<Vec<(FormId,i32)>, String> {
        self.close_production_order(id, false)
    }

    pub fn cancel_production_order(&mut self, id: u64) -> Result<Vec<(FormId,i32)>, String> {
        self.close_production_order(id, true)
    }

    fn close_production_order(&mut self, id: u64, cancel: bool) -> Result<Vec<(FormId,i32)>, String> {
        let o = self.production.orders.get(&id).ok_or("unknown work order")?;
        let final_state = if cancel { OrderState::Cancelled } else { OrderState::Collected };
        if o.state == final_state { return Ok(vec![]) }
        let required = if cancel { OrderState::Working } else { OrderState::Ready };
        if o.state != required { return Err("work order is not available for this action".into()) }
        // At a deadline, completion wins over a last-moment input refund. A host UTC
        // sample is supplied by the host service; callers cannot forge one here.
        if cancel {
            let due = match &o.deadline {
                WorkDeadline::GameHours { due, .. } => self.world.accepted_game_hours().zip(finite(due).ok()).is_some_and(|(h,d)| h >= d),
                WorkDeadline::HostUtc { due, .. } => self.production.last_host_utc.is_some_and(|h| h >= *due),
            };
            if due { return Err("work is due; process completion before cancelling".into()) }
        }
        if self.production_access(o.station)? != (o.station_base, o.cell) {
            return Err("production station identity changed".into());
        }
        ProductionBook::check_stock(o, &self.world)?;
        let revision = self.production.revision.checked_add(1).ok_or("production revisions exhausted")?;
        let moved = self.world.release_production_escrow(o.escrow, id, FormId::PLAYER_REF).map_err(|e| e.to_string())?;
        let o = self.production.orders.get_mut(&id).expect("existing order");
        o.state = final_state;
        o.seal = Inventory::new();
        self.production.revision = revision;
        self.production.schedule();
        Ok(moved)
    }
}

#[cfg(test)]
mod tests;
