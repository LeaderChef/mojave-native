//! Authored tests of the real save guard and retained read-once script answers (#21).

use super::*;
use fnv_game::game::RunCtx;
use fnv_script::vm::XVal;

fn world() -> World {
    let mut world = World::new();
    world.insert_resource(ScriptState::over_no_plugins());
    world.insert_resource(crate::loading::Loading::default());
    world.insert_resource(State::new(PlayState::Playing));
    world.insert_resource(crate::pause::Paused::default());
    world.insert_resource(crate::menu::ActiveMenu::default());
    world
}

fn query(world: &mut World, body: &str) -> XVal {
    world.resource_mut::<ScriptState>().rt.as_mut().unwrap().game
        .run_query(&format!("Begin Function {{ }}\n{body}\nEnd\n"), RunCtx::default()).unwrap()
}

#[test]
fn answered_message_allows_save_and_keeps_the_owners_read_once_result() {
    let mut world = world();
    assert!(not_now(&world).is_none());
    query(&mut world, "MessageBoxEx \"Authored save guard question|Yes|No\"");
    assert!(not_now(&world).is_some(), "an actual unanswered box blocks saving");
    {
        let mut script = world.resource_mut::<ScriptState>();
        let game = &mut script.rt.as_mut().unwrap().game;
        game.press_button(1).unwrap();
        assert!(game.menu.is_none() && game.boxes.is_empty());
        assert_eq!(game.message_box.as_ref().unwrap().pressed, Some(1));
    }
    assert!(not_now(&world).is_none(), "a closed answered box is script bookkeeping, not an open modal");
    // A different source-unit owner cannot consume this answer.
    world.resource_mut::<ScriptState>().rt.as_mut().unwrap().game
        .run_source("GetButtonPressed", RunCtx::default()).unwrap();
    assert_eq!(world.resource::<ScriptState>().rt.as_ref().unwrap().game.message_box.as_ref().unwrap().pressed, Some(1));
    assert_eq!(query(&mut world, "SetFunctionValue (GetButtonPressed)"), XVal::Num(1.0));
    assert_eq!(query(&mut world, "SetFunctionValue (GetButtonPressed)"), XVal::Num(-1.0));
    assert!(not_now(&world).is_none());
}

#[test]
fn save_guard_keeps_unanswered_queued_and_other_script_menus_blocked() {
    let mut world = world();
    query(&mut world, "MessageBoxEx \"First authored warning|OK\"\nMessageBoxEx \"Second authored warning|Yes|No\"");
    {
        let mut script = world.resource_mut::<ScriptState>();
        let game = &mut script.rt.as_mut().unwrap().game;
        assert_eq!(game.boxes.len(), 2);
        game.close_menu();
        assert_eq!(game.boxes.len(), 1);
    }
    assert!(not_now(&world).is_some(), "the next queued box remains live");
    {
        let mut script = world.resource_mut::<ScriptState>();
        let game = &mut script.rt.as_mut().unwrap().game;
        // A transitional frame without an active menu must still protect the queue.
        game.menu = None;
        game.message_box.as_mut().unwrap().pressed = Some(0);
    }
    assert!(not_now(&world).is_some(), "queued content is not made safe by an answered latest-box record");
    {
        let mut script = world.resource_mut::<ScriptState>();
        let game = &mut script.rt.as_mut().unwrap().game;
        game.menu = Some(fnv_game::game::menus::MESSAGE);
        game.close_menu();
    }
    assert!(not_now(&world).is_none());
    {
        let mut script = world.resource_mut::<ScriptState>();
        script.rt.as_mut().unwrap().game.open_menu(fnv_game::game::menus::INVENTORY);
    }
    assert!(not_now(&world).is_some(), "an unrelated open script menu is still protected");
}
