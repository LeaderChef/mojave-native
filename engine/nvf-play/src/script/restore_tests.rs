//! Exercise the actual Runtime restore boundary over synthetic data.

use super::*;

fn state() -> (ScriptState, SimState) {
    let script = ScriptState::over_no_plugins();
    let rt = script.rt.as_ref().unwrap();
    let forms = rt.game.world.forms;
    let refs = rt.game.world.placed;
    let sim = SimState { forms, refs, world: World::new(forms, refs, fnv_sim::SimConfig::default()) };
    (script, sim)
}

#[test]
fn game_validation_failure_preserves_the_current_game_world_and_runtime_flags() {
    let (mut script, mut sim) = state();
    let rt = script.rt.as_mut().unwrap();
    rt.game.ext.new_string("the current game's value".into());
    rt.opening_pending = true;
    rt.new_game_files = true;
    rt.player_cell = Some(FormId(0x100));
    let before_game = rt.game.snapshot();
    let before_world = sim.world.snapshot();
    let mut bad = before_game.clone();
    bad.format = "fnv.game.state/future".into();
    bad.ext.new_string("the rejected saved value".into());
    bad.sim.config.hardcore = true;
    assert!(rt.restore_save(&mut sim, bad).is_err());
    assert_eq!(rt.game.snapshot(), before_game);
    assert_eq!(sim.world.snapshot(), before_world);
    assert!(rt.opening_pending && rt.new_game_files);
    assert_eq!(rt.player_cell, Some(FormId(0x100)));
}

#[test]
fn invalid_world_rng_does_not_partially_apply_the_saved_game() {
    let (mut script, mut sim) = state();
    let rt = script.rt.as_mut().unwrap();
    rt.game.ext.new_string("preserve me".into());
    let before_game = rt.game.snapshot();
    let before_world = sim.world.snapshot();
    let mut bad = before_game.clone(); bad.sim.rng_word_pos = "not an integer".into();
    assert!(rt.restore_save(&mut sim, bad).is_err());
    assert_eq!(rt.game.snapshot(), before_game);
    assert_eq!(sim.world.snapshot(), before_world);
}

#[test]
fn game_validation_failure_does_not_take_the_current_sessions_restart_marker() {
    use fnv_script::vm::XVal;
    let (mut script, mut sim) = state();
    let rt = script.rt.as_mut().unwrap();
    let query = "Begin Function { }\nSetFunctionValue (GetGameRestarted)\nEnd\n";
    assert_eq!(rt.game.run_query(query, Default::default()).unwrap(), XVal::Num(1.0));
    assert_eq!(rt.game.run_query(query, Default::default()).unwrap(), XVal::Num(0.0));
    let mut rejected = rt.game.snapshot(); rejected.format = "fnv.game.state/future".into();
    assert!(rt.restore_save(&mut sim, rejected).is_err());
    assert_eq!(rt.game.run_query(query, Default::default()).unwrap(), XVal::Num(0.0),
        "a failed candidate must not drain the live session before Game::restore accepts it");
}

#[test]
fn a_valid_candidate_applies_the_real_world_and_saved_game_together() {
    let (mut script, mut sim) = state();
    let rt = script.rt.as_mut().unwrap();
    let mut saved = rt.game.snapshot();
    saved.ext.new_string("saved game value".into());
    saved.sim.config.hardcore = true;
    rt.restore_save(&mut sim, saved).unwrap();
    assert!(sim.world.config().hardcore);
    assert!(rt.game.world.config().hardcore, "the between-turn placeholder uses the accepted world's configuration");
    assert_eq!(rt.game.ext.strings.get(&1).map(String::as_str), Some("saved game value"));
    assert!(!rt.opening_pending && !rt.new_game_files);
}

#[test]
fn session_base_edit_is_audited_before_a_funded_candidate_can_replace_live_state() {
    use fnv_game::crafting::RecipeDef;
    use fnv_sim::forms::{ActorBase, ArmorClass, BaseData, BaseForm, Container, Entry, Item, ItemKind};
    use fnv_script::vm::XVal;

    const MATERIAL: FormId = FormId(0x100);
    const STORAGE: FormId = FormId(0x200);
    const BENCH: FormId = FormId(0x201);
    const CATEGORY: FormId = FormId(0x202);
    const RECIPE: FormId = FormId(0x203);
    const CELL: FormId = FormId(0x204);
    let (mut script, _) = state();
    let rt = script.rt.as_mut().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let lo = fnv_esm::LoadOrder::new(&[]).unwrap();
    let schema = fnv_esm::Schema::load_dir(root.join("fnv")).unwrap();
    let functions = root.join("script/functions.json");
    let json = serde_json::from_str(&std::fs::read_to_string(&functions).unwrap()).unwrap();
    let refs: &'static RefTable = Box::leak(Box::new(RefTable::default()));
    let mut data = GameData::build(&lo, &schema, FunctionTable::load(&functions).unwrap(), &json, refs);
    let mut forms = fnv_sim::FormStore::default();
    let material = Item {
        kind: ItemKind::Misc, value: Some(1), money: None, weight: Some(1.0),
        slots: None, projectile: None, health: None, non_playable: false, cant_drop: false,
        armor_class: ArmorClass::Light, power_armor: false, armor_dr: None, armor_dt: None,
        weapon_damage: None, weight_mods: [0.0; 3], drawable: false, not_for_npcs: false,
        thrown: false, quest_item: false, icon: None, placed_count: None, teaches: None,
        cant_be_taken: false, modulates_voice: false, weapon_traits: None,
    };
    for (id, sig, edid, base) in [
        (MATERIAL, *b"MISC", "RestoreMaterial", BaseData::Item(material)),
        (STORAGE, *b"CONT", "MetalBox01Empty", BaseData::Container(Container { respawns: false, entries: vec![] })),
        (BENCH, *b"ACTI", "WorkBench", BaseData::Activator),
        (FormId::PLAYER_BASE, *b"NPC_", "Player", BaseData::Actor(ActorBase { entries: vec![], attributes: Some([5; 7]) })),
    ] {
        forms.edids.insert(edid.to_ascii_lowercase(), id); forms.sigs.insert(id, sig);
        forms.bases.insert(id, BaseForm { id, sig, edid: Some(edid.into()), name: Some(edid.into()),
            script: None, prompt: None, voice_filter: false, data: base });
        data.edids.insert(edid.to_ascii_lowercase(), id); data.edid_of.insert(id, edid.into()); data.sigs.insert(id, sig);
    }
    data.edids.insert("workbenchrecipes".into(), CATEGORY);
    data.edid_of.insert(CATEGORY, "WorkbenchRecipes".into());
    forms.sigs.insert(CATEGORY, *b"RCCT");
    data.cells.insert(CELL, fnv_game::crime::CellDef { owner: None, rank: None, public: true,
        interior: true, off_limits: false, cant_wait: false });
    data.recipes.insert(RECIPE, RecipeDef { form: RECIPE, edid: Some("RestoreRecipe".into()),
        name: "Restore candidate".into(), conditions: vec![], skill: None,
        category: Some(CATEGORY), subcategory: None,
        ingredients: vec![(MATERIAL, 2)], outputs: vec![(MATERIAL, 3)] });
    let forms: &'static fnv_sim::FormStore = Box::leak(Box::new(forms));
    let data: &'static GameData = Box::leak(Box::new(data));
    rt.game = Game::new(data, World::new(forms, refs, fnv_sim::SimConfig::default()), GameConfig::default());
    let mut sim = SimState { forms, refs, world: World::new(forms, refs, fnv_sim::SimConfig::default()) };
    sim.world.get(FormId::PLAYER_REF).unwrap().cell = Some(CELL);
    let station = sim.world.place(BENCH, Some(CELL), [0.0; 3], [0.0; 3], 1).unwrap();
    sim.world.add_item(FormId::PLAYER_REF, MATERIAL, 2).unwrap();
    let saved = with_world(rt, &mut sim, |rt| {
        rt.game.queue_recipe_order(station, CATEGORY, RECIPE, 1).unwrap();
        rt.game.snapshot()
    });
    // The saved stock is valid under its original empty base. The current
    // session's later BaseAddItem edit must be checked before committing it.
    let clean_world = World::restore(forms, refs, &saved.sim).unwrap();
    assert!(Game::restore(data, clean_world, GameConfig::default(), &saved).is_ok());
    assert!(sim.world.edit_base_entries(STORAGE, |entries| entries.push(Entry::new(MATERIAL, 1))));
    let restart = "Begin Function { }\nSetFunctionValue (GetGameRestarted)\nEnd\n";
    assert_eq!(rt.game.run_query(restart, Default::default()).unwrap(), XVal::Num(1.0));
    rt.opening_pending = true; rt.new_game_files = true; rt.player_cell = Some(CELL);
    let before_game = rt.game.snapshot(); let before_world = sim.world.snapshot();
    let before_session = sim.world.session_snapshot();
    assert!(rt.restore_save(&mut sim, saved).is_err());
    assert_eq!(rt.game.snapshot(), before_game); assert_eq!(sim.world.snapshot(), before_world);
    assert_eq!(sim.world.session_snapshot(), before_session);
    assert_eq!(rt.game.run_query(restart, Default::default()).unwrap(), XVal::Num(0.0));
    assert!(rt.opening_pending && rt.new_game_files); assert_eq!(rt.player_cell, Some(CELL));
}
