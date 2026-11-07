//! [design] Recipe-order UI bridge and local single-player host deadline service.
//! Actual stock, eligibility, escrow and saved order transitions belong to fnv-game.
//! This file contains no second stock list, offline clock override or remote authority.

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct OrderView {
    pub id: u64,
    pub text: String,
    pub collect: bool,
    pub cancel: bool,
}

pub(super) fn actual_station(station: Option<FormId>) -> Option<FormId> {
    station.filter(|&station| station != FormId::PLAYER_REF)
}

pub(super) fn station_summary(game: &fnv_game::Game<'_>, station: FormId) -> Option<OrderView> {
    game.production.station_summary(station, game.world.accepted_game_hours()).map(|order| OrderView {
        id: order.id, text: order.text, collect: order.collect, cancel: order.cancel,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn queue_recipe(
    rt: &mut crate::script::Runtime,
    sim: &mut SimState,
    station: Option<FormId>,
    category: FormId,
    recipe: &RecipeDef,
    times: u32,
    sounds: &mut MessageWriter<UiSound>,
    corner: Option<&mut crate::corner_messages::CornerMessages>,
) -> Option<u64> {
    let result = match actual_station(station) {
        Some(station) => crate::script::with_world(rt, sim, |rt| rt.game.queue_recipe_order(station, category, recipe.form, times)),
        None => Err("An actual crafting station is required".to_string()),
    };
    match result {
        Ok(id) => {
            info!("recipe production: queued order {id}: {times} x {:?}; 15 game min per batch", recipe.name);
            sounds.write(UiSound { menu: UiMenu::Recipe, event: UiEvent::Picked });
            if let Some(corner) = corner {
                corner.push(format!("Production order #{id} started").into());
            }
            Some(id)
        }
        Err(why) => {
            info!("recipe production: cannot queue {:?}: {why}", recipe.name);
            sounds.write(UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused });
            None
        }
    }
}

pub(super) fn finish_order(
    rt: &mut crate::script::Runtime,
    sim: &mut SimState,
    id: u64,
    collect: bool,
    sounds: &mut MessageWriter<UiSound>,
    mut corner: Option<&mut crate::corner_messages::CornerMessages>,
) -> bool {
    let forms = sim.forms;
    let verb = if collect { "collected" } else { "cancelled" };
    let (result, notices) = crate::script::with_world(rt, sim, |rt| {
        // Sample again at the actual action boundary: the one-second background
        // throttle is not permission to refund inputs after a host deadline.
        let host_utc = SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|duration| duration.as_secs());
        let needs_host = rt.game.production.orders.get(&id).is_some_and(|order| matches!(&order.deadline, fnv_game::production::WorkDeadline::HostUtc { .. }));
        if needs_host && host_utc.is_none() {
            return (Err("Host UTC is unavailable; this order remains pending".to_string()), Vec::new());
        }
        let notices = rt.game.advance_production_orders(host_utc);
        let result = if collect { rt.game.collect_production_order(id) } else { rt.game.cancel_production_order(id) };
        (result, notices)
    });
    for notice in notices {
        info!("recipe production: {notice}");
        if let Some(corner) = corner.as_deref_mut() {
            corner.push(notice.into());
        }
    }
    match result {
        Ok(moved) => {
            let what = moved.iter().map(|(item, count)| format!("{count} x {}", crate::menu::model::display_name(forms, *item))).collect::<Vec<_>>().join(", ");
            info!("recipe production: {verb} order {id}: {what}");
            sounds.write(UiSound { menu: UiMenu::Recipe, event: if collect { UiEvent::Picked } else { UiEvent::Back } });
            if let Some(corner) = corner {
                corner.push(format!("Production order #{id} {verb}").into());
            }
            true
        }
        Err(why) => {
            info!("recipe production: cannot finish order {id}: {why}");
            sounds.write(UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused });
            false
        }
    }
}

/// Explicit bounded acceptance hook. The test may choose a short duration, never
/// an epoch or elapsed-time sample. Normal Q and its quantity prompt do not use it.
#[allow(clippy::too_many_arguments)]
pub(super) fn queue_host_recipe(
    rt: &mut crate::script::Runtime,
    sim: &mut SimState,
    station: Option<FormId>,
    category: FormId,
    recipe: &RecipeDef,
    sounds: &mut MessageWriter<UiSound>,
    corner: Option<&mut crate::corner_messages::CornerMessages>,
) -> Option<u64> {
    let result = (|| {
        let seconds = std::env::var("NVF_PLAY_TEST_PRODUCTION_HOST_SECONDS").ok()
            .and_then(|value| value.parse::<u64>().ok()).filter(|seconds| (1..=3600).contains(seconds))
            .ok_or("queue-host needs NVF_PLAY_TEST_PRODUCTION_HOST_SECONDS in 1..=3600")?;
        let station = actual_station(station).ok_or("An actual crafting station is required")?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "host UTC is unavailable")?.as_secs();
        crate::script::with_world(rt, sim, |rt| {
            rt.game.queue_production_order(station, category, recipe.form, 1, rt.game.production.next_id,
                fnv_game::production::WorkTiming::HostUtc { now, seconds })
        })
    })();
    match result {
        Ok(id) => {
            if let Some(order) = rt.game.production.orders.get(&id) {
                if let fnv_game::production::WorkDeadline::HostUtc { started, due } = &order.deadline {
                    info!("test aid production host: queued order {id}; started_utc={started}; due_utc={due}; duration_seconds={}", due - started);
                }
            }
            sounds.write(UiSound { menu: UiMenu::Recipe, event: UiEvent::Picked });
            if let Some(corner) = corner {
                corner.push(format!("Test production order #{id} started").into());
            }
            Some(id)
        }
        Err(why) => {
            info!("test aid production host: queue refused: {why}");
            sounds.write(UiSound { menu: UiMenu::Recipe, event: UiEvent::Refused });
            None
        }
    }
}

/// A real-time poll is only a throttle. It never substitutes elapsed wall time for
/// the game's accepted clock and never supplies a caller-controlled test timestamp.
#[derive(Default)]
pub(super) struct HostPoll {
    next: f64,
    clock_failed: bool,
}

/// [design] This executable is the local single-player host. A future multiplayer
/// service must execute the same book transition on its authoritative server instead;
/// this is not networking or a server-time proof. An absent/pending world is not ticked.
pub(super) fn host_orders(
    time: Res<Time<Real>>,
    state: Res<State<PlayState>>,
    pending: Option<Res<crate::save::PendingEngine>>,
    script: Option<ResMut<ScriptState>>,
    mut sim: ResMut<SimState>,
    mut corner: Option<ResMut<crate::corner_messages::CornerMessages>>,
    mut poll: Local<HostPoll>,
) {
    if *state.get() == PlayState::Loading || pending.is_some() {
        return;
    }
    let Some(script) = script.map(|script| script.into_inner()) else { return };
    if script.off.is_some() {
        return;
    }
    let Some(rt) = script.rt.as_mut() else { return };
    if !rt.game.production.has_host_orders() {
        poll.next = 0.0;
        return;
    }
    let real = time.elapsed_secs_f64();
    if real < poll.next {
        return;
    }
    poll.next = real + 1.0;
    let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        if !poll.clock_failed {
            warn!("production host clock unavailable; real-time orders remain pending");
            poll.clock_failed = true;
        }
        return;
    };
    poll.clock_failed = false;
    let notices = crate::script::with_world(rt, &mut sim, |rt| rt.game.advance_production_orders(Some(elapsed.as_secs())));
    for notice in notices {
        info!("production host: {notice}");
        if let Some(corner) = corner.as_deref_mut() {
            corner.push(notice.into());
        }
    }
}

#[cfg(test)]
mod tests;
