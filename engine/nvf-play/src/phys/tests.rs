//! Streaming through the loader, over cells made up in memory (no game data): the world
//! must come out the same whatever order the loader finishes cells in, the same as
//! loading every cell on the main thread, and the same on every run.

use super::*;
use fnv_collide::{CollisionObject, Iso, Motion, Shape};
use fnv_phys::{CharacterInput, PreparedRef, PreparedTerrain, RefPose};
use fnv_sim::{FormStore, RefTable, SimConfig, World};
use fnv_world::land::SIDE;
use fnv_world::WorldIndex;
use prepare::{Entry, RefShapes};

fn sim() -> SimState {
    let forms: &'static FormStore = Box::leak(Box::default());
    let refs: &'static RefTable = Box::leak(Box::default());
    SimState { forms, refs, world: World::new(forms, refs, SimConfig::default()) }
}

fn play_data() -> PlayData {
    let lo: &'static fnv_esm::LoadOrder<'static> = Box::leak(Box::new(fnv_esm::LoadOrder::new(&[]).expect("empty load order")));
    let schema: &'static fnv_esm::Schema = Box::leak(Box::default());
    let index: &'static WorldIndex = Box::leak(Box::default());
    PlayData { lo, schema, index, world: 0x3C, interior: None, eye: [0.0; 3], heading_deg: 0.0, pitch_deg: 0.0, auto_walk: 0.0, start_at_cell_spot: false, packages: false }
}

fn object(half: f32, motion: Motion) -> CollisionObject {
    CollisionObject {
        node: None,
        node_block: 0,
        node_transform: Iso::IDENTITY,
        body_transform: Iso::IDENTITY,
        motion,
        layer: fnv_phys::layers::STATIC,
        mass: 20.0,
        friction: 0.5,
        restitution: 0.0,
        shape: Shape::Box { half_extents: [half; 3] }.into(),
        shell: 0.0,
        material: None,
    }
}

/// A made-up exterior cell: gently sloping terrain, a row of static boxes and a dynamic
/// crate dropped from above (so that stepping depends on when and in what order bodies
/// arrived), with FormIDs derived from the grid square.
fn fake_cell(job: &Job) -> Prepared {
    let PhysCell::Exterior(gx, gy) = job.cell else { panic!("exterior cells only") };
    let params = PhysParams::default();
    let (x0, y0) = (gx as f32 * CELL_SIZE, gy as f32 * CELL_SIZE);
    // A plane rising to the north-east, continuous across cells.
    let heights: Vec<f32> =
        (0..SIDE * SIDE).map(|i| 0.01 * (x0 + (i % SIDE) as f32 * 128.0) + 0.005 * (y0 + (i / SIDE) as f32 * 128.0)).collect();
    let base = 0x0100_0000 + (((gx + 100) as u32) << 12) + (((gy + 100) as u32) << 4);
    let mut refs = Vec::new();
    for i in 0..6u32 {
        let dynamic = i == 5;
        let position = [x0 + 300.0 + i as f32 * 600.0, y0 + 1500.0 + (i % 2) as f32 * 900.0, if dynamic { 400.0 } else { 60.0 }];
        let pl = Placement {
            form_id: base + i,
            sig: *b"REFR",
            flags: 0,
            base: None,
            position,
            rotation: [0.0, 0.0, i as f32 * 0.4],
            scale: 1.0,
            enable_parent: None,
            open_by_default: false,
            teleport: false,
            emittance: None,
            imposter: false,
            primitive: false,
        };
        // As `prepare::cell` does: at the simulation's pose when it holds another.
        let pose = job.pose_of(pl.form_id);
        let pl = prepare::at_pose(pl, pose);
        let obj = object(if dynamic { 15.0 } else { 50.0 }, if dynamic { Motion::Dynamic } else { Motion::Static });
        let prepared = PreparedRef::new(&params, &RefPose::from_placement(&pl), &[obj]);
        let dynamic_local = dynamic.then_some(Iso::IDENTITY);
        // As `prepare::reference` does: built from its destruction model when it has one.
        let model = job.model_of(pl.form_id).map(str::to_string);
        refs.push(Entry { form_id: pl.form_id, placement: Some(pl), shapes: Some(RefShapes { tallies: vec!["fake"], water: Vec::new(), prepared: Some(prepared), dynamic_local, door: None, animated: None, drawn: None, model }) });
    }
    // A pond: a reference with only a water volume (placed water, physics.md §6.6).
    let pond = Placement {
        form_id: base + 6,
        sig: *b"REFR",
        flags: 0,
        base: None,
        position: [x0 + 2000.0, y0 + 3500.0, 0.0],
        rotation: [0.0; 3],
        scale: 1.0,
        enable_parent: None,
        open_by_default: false,
        teleport: false,
        emittance: None,
        imposter: false,
        primitive: false,
    };
    let water = vec![CollisionObject { layer: fnv_phys::layers::WATER, ..object(100.0, Motion::Static) }];
    refs.push(Entry { form_id: pond.form_id, placement: Some(pond), shapes: Some(RefShapes { tallies: vec!["water_volume"], water, prepared: None, dynamic_local: None, door: None, animated: None, drawn: None, model: None }) });
    let terrain = Some(PreparedTerrain::new(&params, (gx, gy), &heights));
    Prepared { cell: job.cell, heights: Some(heights), terrain, refs, times: LoadTimes::default() }
}

/// [`fake_cell`] with its second box keyframed (as a swing trap's held bodies).
fn fake_cell_keyframed(job: &Job) -> Prepared {
    let mut c = fake_cell(job);
    let e = &mut c.refs[1];
    let pl = e.placement.as_ref().expect("placed");
    let prepared = PreparedRef::new(&PhysParams::default(), &RefPose::from_placement(pl), &[object(50.0, Motion::Keyframed)]);
    e.shapes.as_mut().expect("shapes").prepared = Some(prepared);
    c
}

fn state(manual: bool) -> PhysState {
    let loader = if manual { loader::Loader::manual() } else { loader::Loader::default() };
    PhysState { fake: Some(fake_cell), loader, ..Default::default() }
}

fn centre(gx: i32, gy: i32) -> [f32; 3] {
    [(gx as f32 + 0.5) * CELL_SIZE, (gy as f32 + 0.5) * CELL_SIZE, 50.0]
}

/// Bit-identical worlds: every body's state in handle order, and what is loaded.
fn fingerprint(p: &PhysState) -> (u64, Vec<PhysCell>, fnv_phys::Stats) {
    (p.world.state_digest(), p.cells.keys().copied().collect(), p.world.stats())
}

/// FormID of made-up reference `i` of cell (`gx`, `gy`) ([`fake_cell`]).
fn fake_ref(gx: i32, gy: i32, i: u32) -> u32 {
    0x0100_0000 + (((gx + 100) as u32) << 12) + (((gy + 100) as u32) << 4) + i
}

#[test]
fn a_destroyed_reference_collides_as_its_destruction_model_and_back() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = state(true);
    p.load(&ctx, PhysCell::Exterior(0, 0));
    let (car, other) = (fake_ref(0, 0, 1), fake_ref(0, 0, 2));
    let hulk = "Vehicles\\CarHulk01.NIF".to_string();
    assert_eq!(p.built_model(car), None);
    // The stage that names a model: built again from it, the others left alone.
    let built = p.refs_built;
    p.set_destruction_models(&ctx, HashMap::from([(car, hulk.clone())]));
    assert_eq!(p.built_model(car), Some(hulk.as_str()));
    assert!(p.has_collision(car));
    assert_eq!(p.built_model(other), None);
    assert_eq!(p.refs_built, built + 1, "only the destroyed reference is built again");
    // Nothing changed: nothing built.
    p.set_destruction_models(&ctx, HashMap::from([(car, hulk.clone())]));
    assert_eq!(p.refs_built, built + 1);
    // ClearDestruction: the base model's collision again.
    p.set_destruction_models(&ctx, HashMap::new());
    assert_eq!(p.built_model(car), None);
    assert!(p.has_collision(car));
    assert_eq!(p.refs_built, built + 2);
    // A reference whose cell is not loaded gets nothing now, and its model when the
    // cell loads.
    let far = fake_ref(5, 5, 1);
    p.set_destruction_models(&ctx, HashMap::from([(far, hulk.clone())]));
    assert_eq!(p.built_model(far), None);
    assert!(!p.has_collision(far));
    p.load(&ctx, PhysCell::Exterior(5, 5));
    assert_eq!(p.built_model(far), Some(hulk.as_str()));
    // Its cell unloaded: forgotten with it.
    p.unload(PhysCell::Exterior(5, 5));
    assert_eq!(p.built_model(far), None);
}

#[test]
fn a_job_knows_its_references_destruction_models() {
    let job = Job { cell: PhysCell::Exterior(0, 0), refs: vec![1, 5, 9], land: None, poses: Vec::new(), scales: Vec::new(), models: vec![(5, "a.nif".into()), (9, "b.nif".into())] };
    assert_eq!(job.model_of(5), Some("a.nif"));
    assert_eq!(job.model_of(9), Some("b.nif"));
    assert_eq!(job.model_of(1), None);
}

#[test]
fn cells_go_in_in_request_order_whatever_order_they_finish() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let run = |reverse: bool| {
        let mut p = state(true);
        p.stream(&ctx, centre(0, 0), CELLS_PER_FRAME);
        // The cell underfoot went in at once; its eight neighbours wait for the loader.
        assert_eq!(p.cells.keys().copied().collect::<Vec<_>>(), vec![PhysCell::Exterior(0, 0)]);
        assert_eq!(p.loader.pending().count(), 8);
        p.loader.run_manual(reverse);
        for _ in 0..8 {
            p.stream(&ctx, centre(0, 0), CELLS_PER_FRAME);
        }
        assert_eq!(p.cells.len(), 9);
        for _ in 0..30 {
            p.world.step();
        }
        fingerprint(&p)
    };
    let forward = run(false);
    assert_eq!(run(true), forward);
    // The same as loading every cell on the main thread, nearest first.
    let mut sync = state(true);
    for (x, y) in cells::wanted((0, 0)) {
        sync.load(&ctx, PhysCell::Exterior(x, y));
    }
    for _ in 0..30 {
        sync.world.step();
    }
    assert_eq!(fingerprint(&sync), forward);
    // The check can fail: in another order the same cells give another world.
    let mut other = state(true);
    for (x, y) in cells::wanted((0, 0)).into_iter().rev() {
        other.load(&ctx, PhysCell::Exterior(x, y));
    }
    for _ in 0..30 {
        other.world.step();
    }
    assert_ne!(fingerprint(&other), forward);
}

/// Walk east across three cells, streaming as the game does and waiting for the loader
/// thread each frame.
fn walk() -> ((u64, Vec<PhysCell>, fnv_phys::Stats), [f32; 3]) {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = state(false);
    let start = centre(0, 0);
    p.stream(&ctx, start, CELLS_PER_FRAME);
    p.finish_loading(&ctx);
    p.place_player(start, 200.0, 1000.0).expect("ground under the start");
    for _ in 0..900 {
        let feet = p.player.as_ref().unwrap().feet;
        p.stream(&ctx, feet, CELLS_PER_FRAME);
        p.finish_loading(&ctx);
        assert!(p.ready_at(feet));
        let c = p.player.as_mut().unwrap();
        p.world.move_character(c, CharacterInput { velocity: [700.0, 0.0], ..Default::default() });
        p.world.step();
    }
    let feet = p.player.as_ref().unwrap().feet;
    assert!(feet[0] > 2.0 * CELL_SIZE, "walked into the third cell: {feet:?}");
    assert_eq!(p.stats.loaded_inline, 0);
    (fingerprint(&p), feet)
}

#[test]
fn two_runs_are_identical() {
    let a = walk();
    let b = walk();
    assert_eq!(a.0, b.0);
    assert_eq!(a.1.map(f32::to_bits), b.1.map(f32::to_bits));
}

#[test]
fn the_cell_underfoot_never_waits_for_the_loader() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    // A loader that never finishes anything.
    let mut p = state(true);
    p.stream(&ctx, centre(0, 0), CELLS_PER_FRAME);
    p.place_player(centre(0, 0), 200.0, 1000.0).expect("ground");
    assert_eq!(p.stats.loaded_inline, 0, "the start is not a hitch");
    // The player steps into the next cell before the loader delivered it.
    let next = centre(1, 0);
    assert!(!p.ready_at(next));
    p.stream(&ctx, next, CELLS_PER_FRAME);
    assert!(p.ready_at(next));
    assert_eq!(p.stats.loaded_inline, 1);
    // Its request was dropped: the loader's late result is not inserted twice.
    assert!(!p.loader.pending().any(|c| c == PhysCell::Exterior(1, 0)));
    p.loader.run_manual(false);
    let bodies = p.world.stats().refs;
    for _ in 0..20 {
        p.stream(&ctx, next, CELLS_PER_FRAME);
    }
    assert_eq!(p.cells.len(), 12);
    assert_eq!(p.world.stats().refs, bodies + 10 * 6, "ten more cells of six references");
}

#[test]
fn placed_water_comes_and_goes_with_its_cell() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    // Only the cell underfoot goes in: the neighbours wait for a loader that never runs.
    let mut p = state(true);
    let pond = [2000.0, 3500.0, 50.0];
    assert_eq!(p.world.water_surface_for(pond), None);
    p.stream(&ctx, centre(0, 0), CELLS_PER_FRAME);
    assert_eq!(p.world.water_surface_for(pond), Some(100.0), "the box's top");
    assert_eq!(p.stats.counts.get("water_volume_refs"), Some(&1));
    p.unload(PhysCell::Exterior(0, 0));
    assert_eq!(p.world.water_surface_for(pond), None);
}

#[test]
fn cells_left_behind_are_cancelled_and_reset_forgets_requests() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = state(true);
    p.stream(&ctx, centre(0, 0), CELLS_PER_FRAME);
    // Far away before anything arrived: the old requests go, new ones replace them.
    p.stream(&ctx, centre(10, 0), CELLS_PER_FRAME);
    assert!(p.loader.pending().all(|c| matches!(c, PhysCell::Exterior(x, _) if x >= 9)));
    assert_eq!(p.cells.keys().copied().collect::<Vec<_>>(), vec![PhysCell::Exterior(10, 0)]);
    p.loader.run_manual(false);
    for _ in 0..20 {
        p.stream(&ctx, centre(10, 0), CELLS_PER_FRAME);
    }
    assert_eq!(p.cells.len(), 9);
    assert!(p.cells.keys().all(|c| matches!(c, PhysCell::Exterior(x, _) if (9..=11).contains(x))));
    // Door travel: everything requested is forgotten with the cells.
    p.stream(&ctx, centre(20, 0), CELLS_PER_FRAME);
    assert!(p.loader.pending().count() > 0);
    p.reset(Space::Interior(7));
    assert_eq!(p.loader.pending().count(), 0);
    assert!(p.cells.is_empty());
}

/// [`fake_cell`], with square (1, 0) also holding a walkway: a slab 20 units thick whose
/// top is at z 420, reaching 500 units west over the border into square (0, 0).
fn bridge_cell(job: &Job) -> Prepared {
    let mut p = fake_cell(job);
    if job.cell == PhysCell::Exterior(1, 0) {
        let pl = Placement {
            form_id: 0x0200_0001,
            sig: *b"REFR",
            flags: 0,
            base: None,
            position: [CELL_SIZE + 100.0, 0.5 * CELL_SIZE, 400.0],
            rotation: [0.0; 3],
            scale: 1.0,
            enable_parent: None,
            open_by_default: false,
            teleport: false,
            emittance: None,
            imposter: false,
            primitive: false,
        };
        let obj = CollisionObject { shape: Shape::Box { half_extents: [600.0, 600.0, 20.0] }.into(), ..object(0.0, Motion::Static) };
        let prepared = PreparedRef::new(&PhysParams::default(), &RefPose::from_placement(&pl), &[obj]);
        p.refs.push(Entry { form_id: pl.form_id, placement: Some(pl), shapes: Some(RefShapes { tallies: vec!["fake"], water: Vec::new(), prepared: Some(prepared), dynamic_local: None, door: None, animated: None, drawn: None, model: None }) });
    }
    p
}

#[test]
fn a_start_over_a_neighbours_walkway_waits_for_it() {
    // #1103: a save made on the Bison Steve rollercoaster loaded the player on the ground
    // under it. The start's own square goes in at once; the track's, next door, later.
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = PhysState { fake: Some(bridge_cell), loader: loader::Loader::manual(), ..Default::default() };
    // Saved standing on the walkway, in square (0, 0) 300 units from the border.
    let saved = [CELL_SIZE - 300.0, 0.5 * CELL_SIZE, 420.0];
    p.stream(&ctx, saved, CELLS_PER_FRAME);
    assert!(p.ready_at(saved), "the square underfoot is in");
    assert!(!p.ready_around(saved), "the walkway's square is not");
    // Placed now, the start would find only the terrain (about z 51).
    let early = p.place_player(saved, 120.0, 100_000.0).expect("terrain");
    assert!(early.point[2] < 100.0, "{early:?}");
    p.loader.run_manual(false);
    for _ in 0..8 {
        p.stream(&ctx, saved, CELLS_PER_FRAME);
    }
    assert!(p.ready_around(saved));
    let hit = p.place_player(saved, 120.0, 100_000.0).expect("the walkway");
    assert!((hit.point[2] - 420.0).abs() < 1.0, "{hit:?}");
    let feet = p.player.as_ref().unwrap().feet;
    assert!((feet[2] - 420.0).abs() < 5.0, "back at the saved height: {feet:?}");
}

#[test]
fn indoors_the_one_cell_is_all_a_start_waits_for() {
    let mut p = state(true);
    p.space = Space::Interior(7);
    assert!(!p.ready_around([0.0; 3]));
    p.cells.insert(PhysCell::Interior(7), LoadedCell::default());
    assert!(p.ready_around([0.0; 3]));
}

/// The dynamic crate of cell (0, 0) as a placed reference the simulation knows.
fn sim_with_crate() -> (SimState, u32, [f32; 3]) {
    let id = fake_ref(0, 0, 5);
    let pos = [300.0 + 5.0 * 600.0, 1500.0 + 900.0, 400.0];
    (sim_with_placed(id, pos, [0.0, 0.0, 2.0]), id, pos)
}

/// A simulation that places made-up reference `id` ([`fake_cell`]) at `pos`, turned `rot`.
fn sim_with_placed(id: u32, pos: [f32; 3], rot: [f32; 3]) -> SimState {
    let mut table = RefTable::default();
    table.refs.insert(
        FormId(id),
        fnv_sim::refs::PlacedRef {
            id: FormId(id),
            sig: *b"REFR",
            edid: None,
            base: FormId(0x0500_0000),
            cell: None,
            stored_in: None,
            plugin: 0,
            flags: 0,
            in_persistent_group: false,
            pos,
            rot,
            scale: None,
            count: None,
            ammo: None,
            health: None,
            owner: None,
            lock: None,
            teleport: None,
            enable_parent: None,
            linked: None,
            patrol_idle: None,
            patrol_topic: None,
            merchant_container: None,
            marker_reputation: None,
            open_by_default: false,
            primitive: None,
            activate_parents: Vec::new(),
            parent_activate_only: false,
            prompt: None,
            radiation: None,
            radius: None,
            encounter_zone: None,
            level_modifier: None,
        },
    );
    let forms: &'static FormStore = Box::leak(Box::default());
    let refs: &'static RefTable = Box::leak(Box::new(table));
    SimState { forms, refs, world: World::new(forms, refs, SimConfig::default()) }
}

#[test]
fn placed_clutter_moved_by_physics_is_written_back_and_reloads_there() {
    let d = play_data();
    let (mut sim, id, placed) = sim_with_crate();
    let mut p = state(true);
    p.load(&LoadCtx { d: &d, sim: &sim, data: None }, PhysCell::Exterior(0, 0));
    assert!(p.placed_dynamic.get(&id).is_some_and(|c| !c.moved), "tracked, not moved yet");
    assert!(sim.moved_pose(id).is_none());
    let mut written = 0;
    for _ in 0..240 {
        p.world.step();
        written += p.settle_placed(&mut sim.world).len();
    }
    assert!(p.placed_dynamic[&id].moved && written > 0);
    let (pos, rot) = sim.moved_pose(id).expect("written back");
    assert!(pos[2] < placed[2] - 200.0, "fell onto the terrain: {pos:?}");
    // At rest: nothing more to write.
    assert!(p.settle_placed(&mut sim.world).is_empty());
    // Unloaded and loaded again: built at the simulation's pose, not the placement's.
    p.unload(PhysCell::Exterior(0, 0));
    assert!(!p.placed_dynamic.contains_key(&id));
    // (The made-up cells have no records, so the job names the crate itself.)
    let cell = PhysCell::Exterior(0, 0);
    let job = with_poses(Job { cell, refs: vec![id], land: None, poses: Vec::new(), scales: Vec::new(), models: Vec::new() }, &sim);
    p.insert(&LoadCtx { d: &d, sim: &sim, data: None }, fake_cell(&job));
    let b = p.world.ref_bodies(id)[0];
    for i in 0..3 {
        assert!((b.translation[i] - pos[i]).abs() < 1e-2, "{b:?} against {pos:?}");
    }
    assert!(fnv_world::transform::max_diff(&b.rotation, &fnv_world::transform::ref_rotation(rot)) < 1e-4);
    assert!(!p.placed_dynamic[&id].moved, "a fresh start from where it rests");
}

/// A runtime reference made by the same World::place API used by PlaceAtMe, in an
/// indexed, loaded exterior cell. Shapes below are declared in memory: archive/NIF
/// decoding is outside these physics tests.
fn created_pose_fixture(sig: [u8; 4], motion: Motion, multiple: bool) -> (PlayData, SimState, PhysState, u32, Arc<Vec<CollisionObject>>) {
    const CELL: u32 = 0x24F;
    let base = FormId(0x0500_0000);
    let mut forms = FormStore::default();
    forms.sigs.insert(base, sig);
    let forms: &'static FormStore = Box::leak(Box::new(forms));
    let refs: &'static RefTable = Box::leak(Box::default());
    let mut sim = SimState { forms, refs, world: World::new(forms, refs, SimConfig::default()) };
    let id = sim.world.place(base, Some(FormId(CELL)), [100.0, 100.0, 100.0], [0.0; 3], 1).expect("created reference").0;
    assert!(FormId(id).is_dynamic());
    let mut shape = object(10.0, motion);
    shape.shape = Shape::Box { half_extents: [40.0, 5.0, 10.0] }.into();
    let mut shapes = vec![shape.clone()];
    if multiple {
        shape.node_transform.translation = [0.0, 25.0, 0.0];
        shapes.push(shape);
    }
    let shapes = Arc::new(shapes);
    let mut d = play_data();
    let mut index = WorldIndex::default();
    index.cells.insert(CELL, fnv_world::Cell { form_id: CELL, world: Some(d.world), grid: Some((0, 0)), ..Default::default() });
    let mut world = fnv_world::Worldspace { form_id: d.world, ..Default::default() };
    world.grid.insert((0, 0), CELL);
    index.worldspaces.insert(d.world, world);
    d.index = Box::leak(Box::new(index));
    let mut phys = PhysState { worldspace: d.world, ..Default::default() };
    phys.cells.insert(PhysCell::Exterior(0, 0), LoadedCell::default());
    assert!(phys.install_created(&LoadCtx { d: &d, sim: &sim, data: None }, id, "declared-fixture.nif", shapes.clone()));
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert_eq!(phys.world.ref_bodies(id).len(), shapes.len());
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes), "decoded shapes are shared, not copied");
    (d, sim, phys, id, shapes)
}

fn created_pose_hit(phys: &mut PhysState, id: u32, at: [f32; 3]) -> bool {
    phys.ray([at[0], at[1], at[2] + 100.0], [0.0, 0.0, -1.0], 200.0, Filter::default())
        .is_some_and(|hit| matches!(hit.owner, Owner::Ref { form_id, .. } if form_id == id))
}

/// A loaded interior and real runtime reference; only decoded NIF shapes are supplied
/// in memory. Scale changes go through the production sync_ref/install_created path.
fn created_scale_fixture(scale: f32, motion: Motion, local: Iso) -> (PlayData, SimState, PhysState, u32, Arc<Vec<CollisionObject>>) {
    let (mut d, mut sim, mut phys, id, _) = created_pose_fixture(*b"ACTI", motion, false);
    phys.move_created(id, None);
    phys.cells.clear();
    const CELL: u32 = 0x250;
    let mut index = WorldIndex::default();
    index.cells.insert(CELL, fnv_world::Cell { form_id: CELL, ..Default::default() });
    d.index = Box::leak(Box::new(index));
    d.interior = Some(CELL);
    phys.space = Space::Interior(CELL);
    phys.cells.insert(PhysCell::Interior(CELL), LoadedCell::default());
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.cell = Some(FormId(CELL));
        s.scale = Some(scale);
    }
    let mut shape = object(10.0, motion);
    shape.shape = Shape::Box { half_extents: [40.0, 5.0, 10.0] }.into();
    shape.body_transform = local;
    let shapes = Arc::new(vec![shape]);
    phys.world.remove_ref(id);
    phys.forget_created(id);
    assert!(phys.install_created(&LoadCtx { d: &d, sim: &sim, data: None }, id, "declared-scaled-fixture.nif", shapes.clone()));
    phys.move_created(id, Some(PhysCell::Interior(CELL)));
    (d, sim, phys, id, shapes)
}

#[test]
fn created_scale_initial_install_has_scaled_ray_geometry() {
    let (_, sim, mut phys, id, _) = created_scale_fixture(2.0, Motion::Static, Iso::IDENTITY);
    let at = sim.world.peek(FormId(id)).unwrap().pos;
    assert!(created_pose_hit(&mut phys, id, [at[0] + 70.0, at[1], at[2]]), "initial install must use the saved reference scale");
    assert!(!created_pose_hit(&mut phys, id, [at[0] + 90.0, at[1], at[2]]));
}

#[test]
fn created_scale_only_sync_rebuilds_shrunk_and_enlarged_geometry() {
    let (d, mut sim, mut phys, id, shapes) = created_scale_fixture(1.0, Motion::Static, Iso::IDENTITY);
    let at = sim.world.peek(FormId(id)).unwrap().pos;
    assert!(created_pose_hit(&mut phys, id, [at[0] + 30.0, at[1], at[2]]));
    sim.world.get(FormId(id)).unwrap().scale = Some(0.5);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!created_pose_hit(&mut phys, id, [at[0] + 30.0, at[1], at[2]]), "old unscaled geometry must go");
    assert!(created_pose_hit(&mut phys, id, [at[0] + 15.0, at[1], at[2]]));
    sim.world.get(FormId(id)).unwrap().scale = Some(2.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(created_pose_hit(&mut phys, id, [at[0] + 70.0, at[1], at[2]]));
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes), "no shape copy or model reread is needed to rescale");
    assert_eq!(phys.world.ref_bodies(id).len(), 1);
}

#[test]
fn created_scale_zero_removes_then_positive_restores_and_unload_cleans_cache() {
    let (d, mut sim, mut phys, id, shapes) = created_scale_fixture(1.0, Motion::Static, Iso::IDENTITY);
    for scale in [0.0, crate::sim::MIN_DRAWN_SCALE] {
        sim.world.get(FormId(id)).unwrap().scale = Some(scale);
        phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
        assert!(!phys.has_collision(id), "minimum drawn scales have no collision");
        assert!(!phys.created.contains_key(&id), "a suspended shape cannot settle a removed body");
    }
    sim.world.get(FormId(id)).unwrap().scale = Some(1.5);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(created_pose_hit(&mut phys, id, [150.0, 100.0, 100.0]), "positive scale restores scaled geometry without another archive read");
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
    sim.world.get(FormId(id)).unwrap().scale = Some(0.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    phys.unload(PhysCell::Interior(0x250));
    assert!(!phys.created_objects.contains_key(&id), "zero-scale shape handles must unload with their cell");
    assert!(!phys.ref_cell.contains_key(&id));
}

#[test]
fn created_scale_initial_zero_does_not_need_model_io() {
    let (d, mut sim, mut phys, id, _) = created_scale_fixture(1.0, Motion::Static, Iso::IDENTITY);
    phys.world.remove_ref(id);
    phys.forget_created(id);
    sim.world.get(FormId(id)).unwrap().scale = Some(0.0);
    // The fixture has no base-model/NIF record. Zero may not try to resolve it.
    assert!(!phys.add_created(&LoadCtx { d: &d, sim: &sim, data: None }, id));
    assert!(!phys.has_collision(id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.stats.counts.contains_key("created_no_model"));
    assert!(!phys.stats.counts.contains_key("awaiting_collision"));
}

#[test]
fn created_scale_zero_disable_discards_suspended_shapes_and_off_state() {
    let (d, mut sim, mut phys, id, _) = created_scale_fixture(1.0, Motion::Keyframed, Iso::IDENTITY);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    sim.world.get(FormId(id)).unwrap().scale = Some(0.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(phys.created_objects.contains_key(&id));
    assert!(phys.created_suspended_off.contains(&id));
    sim.world.set_enabled(FormId(id), false).unwrap();
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.created_suspended_off.contains(&id));
    assert!(!phys.created.contains_key(&id));
    assert!(!phys.built_at.contains_key(&id));
    assert!(!phys.ref_cell.contains_key(&id));
    sim.world.get(FormId(id)).unwrap().scale = Some(2.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!phys.has_collision(id), "a later scale does not revive a disabled reference");
}

#[test]
fn created_scale_sync_preserves_keyframed_collision_off() {
    let (d, mut sim, mut phys, id, _) = created_scale_fixture(1.0, Motion::Keyframed, Iso::IDENTITY);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    sim.world.get(FormId(id)).unwrap().scale = Some(2.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 0, "rescaling does not turn a held keyframed body solid");
    assert!(!created_pose_hit(&mut phys, id, [170.0, 100.0, 100.0]), "off collision stays off");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, true), 1);
    assert!(created_pose_hit(&mut phys, id, [170.0, 100.0, 100.0]), "the rescaled body can become solid again");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    sim.world.get(FormId(id)).unwrap().scale = Some(0.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    sim.world.get(FormId(id)).unwrap().scale = Some(1.5);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 0, "zero-to-positive restoration keeps the previous off state too");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, true), 1);
}

#[test]
fn created_scale_repose_zero_preserves_disabled_keyframed_collision() {
    let (d, mut sim, mut phys, id, shapes) = created_scale_fixture(1.0, Motion::Keyframed, Iso::IDENTITY);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.pos[0] += 400.0;
        s.scale = Some(0.0);
    }
    // Normal ECS synchronization handles changed IDs before moved IDs. Exercise the
    // public methods in the opposite order too: the off-state invariant must survive
    // a pose rebuild that suspends rather than installs its zero-scale collision.
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!phys.has_collision(id));
    sim.world.get(FormId(id)).unwrap().scale = Some(1.5);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(phys.has_collision(id), "the positive scale restores the existing reference");
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 0, "restoring a moved zero-scale reference must not revive its previously disabled collision");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, true), 1, "the body is retained and can explicitly be made solid");
    assert!(created_pose_hit(&mut phys, id, [550.0, 100.0, 100.0]), "restored geometry is scaled and at the new position");
    assert!(!created_pose_hit(&mut phys, id, [100.0, 100.0, 100.0]), "the old position remains empty");
}

/// Install through production, then put the real body at the expected scaled local
/// offset. This isolates body->ref inversion from the initial install-scale defect.
fn created_scale_offset_body() -> (PlayData, SimState, PhysState, u32, [f32; 3]) {
    let local = Iso { translation: [12.0, -5.0, 7.0], rotation: fnv_world::transform::ref_rotation([0.1, 0.2, 0.3]) };
    let (d, mut sim, mut phys, id, shapes) = created_scale_fixture(2.0, Motion::Dynamic, local);
    let s = sim.world.get(FormId(id)).unwrap();
    s.rot = [0.0, 0.0, 0.5];
    let at = s.pos;
    phys.world.remove_ref(id);
    let pose = RefPose { form_id: id, position: at, rotation: fnv_world::transform::ref_rotation(s.rot), scale: 2.0 };
    phys.world.add_ref(&pose, &shapes);
    (d, sim, phys, id, at)
}

#[test]
fn created_scale_dynamic_settle_uses_built_scale_without_local_offset_jump() {
    let (d, mut sim, mut phys, id, at) = created_scale_offset_body();
    assert!(phys.settle_created(d.index, d.world, &mut sim.world).is_empty(), "a freshly scaled body has not moved its reference");
    let now = sim.world.peek(FormId(id)).unwrap();
    assert!((Vec3::from_array(now.pos) - Vec3::from_array(at)).length() < 1e-3);
    // An unsynchronised next script scale cannot change how the existing body is read.
    sim.world.get(FormId(id)).unwrap().scale = Some(3.0);
    assert!(phys.settle_created(d.index, d.world, &mut sim.world).is_empty(), "use built scale, not the pending simulation scale");
    // A physical displacement must still write the reference back, without adding
    // either the old or the pending scale's local offset to that displacement.
    let to = [at[0] + 55.0, at[1] - 10.0, at[2] + 20.0];
    let s = sim.world.peek(FormId(id)).unwrap();
    let pose = RefPose { form_id: id, position: to, rotation: fnv_world::transform::ref_rotation(s.rot), scale: 2.0 };
    let shapes = phys.created_objects[&id].clone();
    phys.world.remove_ref(id);
    phys.world.add_ref(&pose, &shapes);
    assert_eq!(phys.settle_created(d.index, d.world, &mut sim.world), vec![id]);
    assert!((Vec3::from_array(sim.world.peek(FormId(id)).unwrap().pos) - Vec3::from_array(to)).length() < 1e-3);
    assert_eq!(sim.world.peek(FormId(id)).unwrap().scale, Some(3.0), "pose write-back does not overwrite a saved script scale");
}

#[test]
fn created_scale_dynamic_repose_does_not_rebuild_a_scaled_local_offset() {
    let (d, sim, mut phys, id, _) = created_scale_offset_body();
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(phys.reposes, 0, "scaled local translation is not a script movement");
    assert!(phys.has_collision(id));
}

const CROSS_A: u32 = 0x260;
const CROSS_B: u32 = 0x261;
const CROSS_I: u32 = 0x262;
const CROSS_J: u32 = 0x263;
const CROSS_OTHER_WORLD_CELL: u32 = 0x264;

/// The same declared decoded shapes and World::place reference as pose tests, now
/// registered through production sync_ref in a real, indexed loaded exterior cell.
/// Only the initial cell/shape setup is supplied by the fixture; no sync_ref call is
/// injected after the tested script move, which arrives through RefRepose alone.
fn created_cross_cell_fixture(sig: [u8; 4], motion: Motion, multiple: bool, load_b: bool, at: [f32; 3]) -> (PlayData, SimState, PhysState, u32, Arc<Vec<CollisionObject>>) {
    let (mut d, mut sim, mut phys, id, shapes) = created_pose_fixture(sig, motion, multiple);
    phys.move_created(id, None);
    phys.cells.clear();
    let mut index = WorldIndex::default();
    for (cell, world, grid) in [
        (CROSS_A, Some(d.world), Some((0, 0))),
        (CROSS_B, Some(d.world), Some((1, 0))),
        (CROSS_I, None, None),
        (CROSS_J, None, None),
        (CROSS_OTHER_WORLD_CELL, Some(0x99), Some((0, 0))),
    ] {
        index.cells.insert(cell, fnv_world::Cell { form_id: cell, world, grid, ..Default::default() });
    }
    let mut world = fnv_world::Worldspace { form_id: d.world, ..Default::default() };
    world.grid.insert((0, 0), CROSS_A);
    world.grid.insert((1, 0), CROSS_B);
    index.worldspaces.insert(d.world, world);
    d.index = Box::leak(Box::new(index));
    phys.worldspace = d.world;
    phys.space = Space::Exterior;
    phys.cells.insert(PhysCell::Exterior(0, 0), LoadedCell::default());
    if load_b {
        phys.cells.insert(PhysCell::Exterior(1, 0), LoadedCell::default());
    }
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.cell = Some(FormId(CROSS_A));
        s.pos = at;
    }
    phys.world.remove_ref(id);
    phys.forget_created(id);
    assert!(phys.install_created(&LoadCtx { d: &d, sim: &sim, data: None }, id, "declared-cross-cell-fixture.nif", shapes.clone()));
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert_eq!(phys.ref_cell.get(&id), Some(&PhysCell::Exterior(0, 0)));
    assert_eq!(phys.cells[&PhysCell::Exterior(0, 0)].refs, [id]);
    (d, sim, phys, id, shapes)
}

#[test]
fn created_cross_cell_loaded_destination_survives_source_unload() {
    for (sig, motion, multiple) in [(*b"STAT", Motion::Static, false), (*b"ACTI", Motion::Keyframed, false), (*b"CONT", Motion::Static, true)] {
        let (d, mut sim, mut phys, id, shapes) = created_cross_cell_fixture(sig, motion, multiple, true, [100.0; 3]);
        let to = [CELL_SIZE + 100.0, 100.0, 100.0];
        {
            let s = sim.world.get(FormId(id)).unwrap();
            s.cell = Some(FormId(CROSS_B));
            s.pos = to;
        }
        phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
        assert!(created_pose_hit(&mut phys, id, to), "the moved reference collides in B: {sig:?}");
        assert!(!created_pose_hit(&mut phys, id, [100.0; 3]));
        assert_eq!(phys.ref_cell.get(&id), Some(&PhysCell::Exterior(1, 0)), "collision owner moves with the reference: {sig:?}");
        assert!(!phys.cells[&PhysCell::Exterior(0, 0)].refs.contains(&id));
        assert_eq!(phys.cells[&PhysCell::Exterior(1, 0)].refs.iter().filter(|&&r| r == id).count(), 1);
        phys.unload(PhysCell::Exterior(0, 0));
        assert!(created_pose_hit(&mut phys, id, to), "unloading A cannot delete B's body");
        assert_eq!(phys.world.ref_bodies(id).len(), shapes.len());
        assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
        // Duplicate move notifications do not create duplicate cell ownership/bodies.
        phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 1.0);
        assert_eq!(phys.cells[&PhysCell::Exterior(1, 0)].refs.iter().filter(|&&r| r == id).count(), 1);
        assert_eq!(phys.world.ref_bodies(id).len(), shapes.len());
        phys.unload(PhysCell::Exterior(1, 0));
        assert!(!phys.has_collision(id));
        assert!(!phys.created_objects.contains_key(&id));
        assert!(!phys.ref_cell.contains_key(&id));
        assert_eq!(sim.world.peek(FormId(id)).unwrap().cell, Some(FormId(CROSS_B)), "cell unload does not delete the simulation reference");
    }
}

#[test]
fn created_cross_cell_small_boundary_move_keeps_destination_while_pose_debounces() {
    let start = [CELL_SIZE - 4.0, 100.0, 100.0];
    let (d, mut sim, mut phys, id, shapes) = created_cross_cell_fixture(*b"STAT", Motion::Static, false, true, start);
    let to = [CELL_SIZE + 4.0, 100.0, 100.0];
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.cell = Some(FormId(CROSS_B));
        s.pos = to;
    }
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(phys.ref_cell.get(&id), Some(&PhysCell::Exterior(1, 0)), "ownership cannot wait for small pose movement to settle");
    assert!(!phys.cells[&PhysCell::Exterior(0, 0)].refs.contains(&id));
    phys.unload(PhysCell::Exterior(0, 0));
    assert!(phys.has_collision(id));
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
    phys.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, super::REPOSE_SETTLE + 0.01);
    assert!(phys.reposes <= 1, "a cell-boundary move remains a bounded geometry update");
    assert!((phys.world.ref_bodies(id)[0].translation[0] - to[0]).abs() < 1e-3);
    assert!(created_pose_hit(&mut phys, id, [to[0] + 39.0, to[1], to[2]]));
    assert!(!created_pose_hit(&mut phys, id, [start[0] - 39.0, start[1], start[2]]));
}

#[test]
fn created_cross_cell_unloaded_destination_discards_old_collision_and_owner() {
    let (d, mut sim, mut phys, id, _) = created_cross_cell_fixture(*b"CONT", Motion::Static, true, false, [100.0; 3]);
    let to = [CELL_SIZE + 100.0, 100.0, 100.0];
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.cell = Some(FormId(CROSS_B));
        s.pos = to;
    }
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert!(!phys.has_collision(id), "a script move cannot install a collider in an unloaded destination");
    assert!(!phys.ref_cell.contains_key(&id));
    assert!(!phys.cells[&PhysCell::Exterior(0, 0)].refs.contains(&id));
    assert!(!phys.created.contains_key(&id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.created_suspended_off.contains(&id));
    assert!(!phys.repose_waiting(id));
    let saved = sim.world.peek(FormId(id)).unwrap();
    assert_eq!(saved.cell, Some(FormId(CROSS_B)));
    assert_eq!(saved.pos, to);
    assert!(dropped::present(saved), "physical unload is not removal of the logical reference");
}

#[test]
fn created_cross_cell_same_pose_other_space_removes_old_collision() {
    for target in [CROSS_I, CROSS_OTHER_WORLD_CELL, CROSS_J, 0xDEAD] {
        let (d, mut sim, mut phys, id, _) = created_cross_cell_fixture(*b"ACTI", Motion::Keyframed, false, false, [100.0; 3]);
        let source = if target == CROSS_J {
            phys.space = Space::Interior(CROSS_I);
            phys.cells.insert(PhysCell::Interior(CROSS_I), LoadedCell::default());
            sim.world.get(FormId(id)).unwrap().cell = Some(FormId(CROSS_I));
            phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
            PhysCell::Interior(CROSS_I)
        } else {
            PhysCell::Exterior(0, 0)
        };
        assert_eq!(phys.ref_cell.get(&id), Some(&source));
        let before = sim.world.peek(FormId(id)).unwrap().pos;
        sim.world.get(FormId(id)).unwrap().cell = Some(FormId(target));
        phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
        assert!(!phys.has_collision(id), "cell/space changes matter even with unchanged position and rotation: {target:08X}");
        assert!(!phys.ref_cell.contains_key(&id));
        assert!(!phys.cells[&source].refs.contains(&id));
        assert!(!phys.created_objects.contains_key(&id));
        assert_eq!(sim.world.peek(FormId(id)).unwrap().pos, before);
        assert_eq!(sim.world.peek(FormId(id)).unwrap().cell, Some(FormId(target)));
    }
}

#[test]
fn created_cross_cell_zero_suspended_transfer_keeps_destination_and_off_state() {
    let (d, mut sim, mut phys, id, shapes) = created_cross_cell_fixture(*b"ACTI", Motion::Keyframed, false, true, [100.0; 3]);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    sim.world.get(FormId(id)).unwrap().scale = Some(0.0);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!phys.has_collision(id));
    assert!(phys.created_suspended_off.contains(&id));
    let to = [CELL_SIZE + 100.0, 100.0, 100.0];
    {
        let s = sim.world.get(FormId(id)).unwrap();
        s.cell = Some(FormId(CROSS_B));
        s.pos = to;
    }
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(phys.ref_cell.get(&id), Some(&PhysCell::Exterior(1, 0)), "a suspended reference still has a loaded-cell owner");
    assert!(!phys.cells[&PhysCell::Exterior(0, 0)].refs.contains(&id));
    assert_eq!(phys.cells[&PhysCell::Exterior(1, 0)].refs.iter().filter(|&&r| r == id).count(), 1);
    phys.unload(PhysCell::Exterior(0, 0));
    assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
    assert!(phys.created_suspended_off.contains(&id));
    sim.world.get(FormId(id)).unwrap().scale = Some(1.5);
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(phys.has_collision(id));
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 0, "restoration keeps the off state across source-cell unload");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, true), 1);
    assert!(created_pose_hit(&mut phys, id, [to[0] + 50.0, to[1], to[2]]));
    phys.unload(PhysCell::Exterior(1, 0));
    assert!(!phys.has_collision(id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.created_suspended_off.contains(&id));
}

#[test]
fn created_cross_cell_repose_does_not_restore_an_absent_collider() {
    let (d, mut sim, mut phys, id, _) = created_cross_cell_fixture(*b"STAT", Motion::Static, false, true, [100.0; 3]);
    // A body removed for fall-through is not restored just because a later pose
    // notification arrives. Ownership and body installation remain separate actions.
    phys.world.remove_ref(id);
    phys.forget_created(id);
    sim.world.get(FormId(id)).unwrap().pos[0] += 400.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert!(!phys.has_collision(id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.stats.counts.contains_key("created_no_model"), "owner synchronization does not begin a model lookup");
    assert!(!phys.stats.counts.contains_key("awaiting_collision"));
    assert_eq!(phys.reposes, 0);
    assert_eq!(phys.ref_cell.get(&id), Some(&PhysCell::Exterior(0, 0)));
}

#[test]
fn created_fixed_pose_rebuilds_after_large_move_and_rotation() {
    for (sig, motion, multiple) in [(*b"STAT", Motion::Static, false), (*b"ACTI", Motion::Keyframed, false), (*b"CONT", Motion::Static, true)] {
        let (d, mut sim, mut phys, id, shapes) = created_pose_fixture(sig, motion, multiple);
        let old = sim.world.peek(FormId(id)).unwrap().pos;
        assert!(created_pose_hit(&mut phys, id, [old[0] + 30.0, old[1], old[2]]));
        let to = [old[0] + 400.0, old[1], old[2]];
        let angles = [0.0, 0.0, std::f32::consts::FRAC_PI_2];
        {
            let s = sim.world.get(FormId(id)).unwrap();
            s.pos = to;
            s.rot = angles;
        }
        phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
        assert!(!created_pose_hit(&mut phys, id, old), "old collider must be removed: {sig:?}");
        assert!(created_pose_hit(&mut phys, id, [to[0], to[1] + 30.0, to[2]]), "new rotated long side must collide: {sig:?}");
        assert!(!created_pose_hit(&mut phys, id, [to[0] + 30.0, to[1], to[2]]), "unrotated long side must not remain: {sig:?}");
        let bodies = phys.world.ref_bodies(id);
        assert_eq!(bodies.len(), shapes.len());
        assert!((bodies[0].translation[0] - to[0]).abs() < 1e-3);
        assert!(fnv_world::transform::max_diff(&bodies[0].rotation, &fnv_world::transform::ref_rotation(angles)) < 1e-4);
        assert!(Arc::ptr_eq(&phys.created_objects[&id], &shapes));
        assert_eq!(phys.reposes, 1);
        phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 1.0);
        assert_eq!(phys.reposes, 1, "already at the requested pose");
    }
}

#[test]
fn created_fixed_pose_small_moves_debounce_then_rebuild() {
    let (d, mut sim, mut phys, id, _) = created_pose_fixture(*b"STAT", Motion::Static, false);
    let old = sim.world.peek(FormId(id)).unwrap().pos;
    sim.world.get(FormId(id)).unwrap().pos[1] += 8.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert!(phys.repose_waiting(id));
    assert_eq!(phys.reposes, 0);
    phys.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, super::REPOSE_SETTLE * 0.9);
    assert!(created_pose_hit(&mut phys, id, [old[0], old[1] - 4.0, old[2]]));
    sim.world.get(FormId(id)).unwrap().pos[1] += 4.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.2);
    phys.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, 0.2 + super::REPOSE_SETTLE * 0.9);
    assert_eq!(phys.reposes, 0, "continued dragging postpones the small rebuild");
    phys.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, 0.2 + super::REPOSE_SETTLE + 0.01);
    assert!(!phys.repose_waiting(id));
    assert_eq!(phys.reposes, 1);
    assert!(!created_pose_hit(&mut phys, id, old));
    assert!(created_pose_hit(&mut phys, id, [old[0], old[1] + 12.0, old[2]]));
}

#[test]
fn created_fixed_pose_disable_clears_queued_collision() {
    let (d, mut sim, mut phys, id, _) = created_pose_fixture(*b"ACTI", Motion::Keyframed, false);
    sim.world.get(FormId(id)).unwrap().pos[1] += 8.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert!(phys.repose_waiting(id));
    sim.world.set_enabled(FormId(id), false).unwrap();
    phys.sync_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id);
    assert!(!phys.has_collision(id));
    assert!(!phys.created.contains_key(&id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.built_at.contains_key(&id));
    assert!(!phys.repose_waiting(id));
    phys.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, 1.0);
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 1.0);
    assert!(!phys.has_collision(id), "a queued pose may not revive a disabled object");
}

#[test]
fn created_fixed_pose_unload_discards_cached_shapes() {
    let (d, mut sim, mut phys, id, _) = created_pose_fixture(*b"CONT", Motion::Static, true);
    let cell = PhysCell::Exterior(0, 0);
    phys.cells.insert(cell, LoadedCell { refs: vec![id], heights: None });
    phys.ref_cell.insert(id, cell);
    sim.world.get(FormId(id)).unwrap().pos[0] += 8.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    phys.unload(cell);
    assert!(!phys.has_collision(id));
    assert!(!phys.created_objects.contains_key(&id));
    assert!(!phys.built_at.contains_key(&id));
    assert!(!phys.repose_waiting(id));
    assert!(!phys.ref_cell.contains_key(&id));
}

#[test]
fn created_fixed_pose_rebuild_keeps_keyframed_collision_off() {
    let (d, mut sim, mut phys, id, _) = created_pose_fixture(*b"ACTI", Motion::Keyframed, false);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 1);
    sim.world.get(FormId(id)).unwrap().pos[0] += 400.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(phys.reposes, 1);
    assert_eq!(phys.world.set_ref_keyframed_solid(id, false), 0, "already off after rebuilding");
    assert_eq!(phys.world.set_ref_keyframed_solid(id, true), 1, "body still exists and can become solid again");
}

#[test]
fn created_fixed_pose_preserves_multiple_dynamic_body_policy() {
    let (d, mut sim, mut phys, id, _) = created_pose_fixture(*b"CONT", Motion::Dynamic, true);
    let before = phys.world.ref_bodies(id);
    sim.world.get(FormId(id)).unwrap().pos[0] += 400.0;
    phys.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(phys.reposes, 0, "multiple moving bodies still have no single reference pose");
    assert_eq!(phys.world.ref_bodies(id), before, "the fixed-body fix does not rebuild moving multi-body clutter");
}

/// A script's move of a reference whose collision physics never moves (a fixed box, as
/// a STAT's) builds it again at the simulation's pose, not left at the old one (issue
/// #1466); a repose where it already stands builds nothing.
#[test]
fn a_script_moved_fixed_reference_collides_at_its_new_pose() {
    let d = play_data();
    let id = fake_ref(0, 0, 1);
    let placed = [300.0 + 600.0, 1500.0 + 900.0, 60.0];
    let mut sim = sim_with_placed(id, placed, [0.0, 0.0, 0.4]);
    let mut p = state(true);
    p.load(&LoadCtx { d: &d, sim: &sim, data: None }, PhysCell::Exterior(0, 0));
    assert!(p.has_collision(id) && !p.placed_dynamic.contains_key(&id), "fixed collision, not one dynamic body");
    let built = p.refs_built;
    // Told of a move it has not made: nothing built.
    p.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(p.refs_built, built);
    // SetPos and SetAngle.
    let (to, angles) = ([placed[0] + 400.0, placed[1] - 200.0, placed[2] + 30.0], [0.0, 0.0, 1.2]);
    {
        let s = sim.world.get(FormId(id)).expect("placed");
        s.pos = to;
        s.rot = angles;
    }
    p.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(p.refs_built, built + 1, "built again");
    let bodies = p.world.ref_bodies(id);
    assert_eq!(bodies.len(), 1);
    for i in 0..3 {
        assert!((bodies[0].translation[i] - to[i]).abs() < 1e-2, "{:?} against {to:?}", bodies[0]);
    }
    assert!(fnv_world::transform::max_diff(&bodies[0].rotation, &fnv_world::transform::ref_rotation(angles)) < 1e-4);
    // Where it was built now: nothing more.
    p.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(p.refs_built, built + 1);
}

/// A script that turns a fixed reference a little every frame (Rotate, 10 degrees a
/// second at 60 frames a second) does not build its collision again every frame: only
/// when it strays [`super::REPOSE_NOW_ANGLE`] from its build, and once when it rests.
#[test]
fn a_reference_turned_every_frame_is_built_again_a_bounded_number_of_times() {
    let d = play_data();
    let id = fake_ref(0, 0, 1);
    let placed = [300.0 + 600.0, 1500.0 + 900.0, 60.0];
    let mut sim = sim_with_placed(id, placed, [0.0, 0.0, 0.4]);
    let mut p = state(true);
    p.load(&LoadCtx { d: &d, sim: &sim, data: None }, PhysCell::Exterior(0, 0));
    assert!(p.has_collision(id) && !p.placed_dynamic.contains_key(&id));
    let (dt, frames) = (1.0 / 60.0, 180);
    let step = 10f32.to_radians() * dt as f32;
    for f in 1..=frames {
        sim.world.get(FormId(id)).expect("placed").rot[2] += step;
        let now = f as f64 * dt;
        let ctx = LoadCtx { d: &d, sim: &sim, data: None };
        p.repose_ref(&ctx, id, now);
        p.flush_reposes(&ctx, now);
    }
    // 30 degrees in 3 s: twice past 15 degrees from the last build, at most.
    assert!(p.reposes <= 2, "built again {} times in {frames} frames", p.reposes);
    assert!(p.repose_waiting(id), "the last turn waits for it to rest");
    // It rests: built once more, where the script left it.
    let rest = frames as f64 * dt + super::REPOSE_SETTLE;
    let before = p.reposes;
    p.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, rest);
    assert_eq!(p.reposes, before + 1);
    assert!(!p.repose_waiting(id));
    let rot = fnv_world::transform::ref_rotation([0.0, 0.0, 0.4 + step * frames as f32]);
    assert!(fnv_world::transform::max_diff(&p.world.ref_bodies(id)[0].rotation, &rot) < 1e-3);
    p.flush_reposes(&LoadCtx { d: &d, sim: &sim, data: None }, rest + 1.0);
    assert_eq!(p.reposes, before + 1, "nothing more once built where it rests");
}

/// Keyframed collision switched off (a swing trap's held bodies after let-go) stays off
/// when a script's move builds it again.
#[test]
fn a_switched_off_collision_stays_off_when_built_again() {
    let d = play_data();
    let id = fake_ref(0, 0, 1);
    let placed = [300.0 + 600.0, 1500.0 + 900.0, 60.0];
    let mut sim = sim_with_placed(id, placed, [0.0, 0.0, 0.4]);
    let mut p = PhysState { fake: Some(fake_cell_keyframed), ..state(true) };
    p.load(&LoadCtx { d: &d, sim: &sim, data: None }, PhysCell::Exterior(0, 0));
    assert_eq!(p.world.set_ref_keyframed_solid(id, false), 1, "its keyframed box switched off");
    sim.world.get(FormId(id)).expect("placed").pos[0] += 400.0;
    p.repose_ref(&LoadCtx { d: &d, sim: &sim, data: None }, id, 0.0);
    assert_eq!(p.reposes, 1);
    assert_eq!(p.world.set_ref_keyframed_solid(id, false), 0, "still off after the build");
    assert_eq!(p.world.set_ref_keyframed_solid(id, true), 1, "and there to switch on again");
}

/// A STAT's dynamic bodies are fixed (the saloon's motorcycles, playtest list 5 item 59);
/// a moveable static's stay dynamic.
#[test]
fn statics_never_move_whatever_their_model_says() {
    let model = || vec![object(10.0, Motion::Dynamic), object(5.0, Motion::Keyframed)];
    let mut tallies = Vec::new();
    let mut stat = model();
    prepare::fix_static_base(Some(*b"STAT"), &mut stat, &mut tallies);
    assert_eq!(stat.iter().map(|o| o.motion).collect::<Vec<_>>(), [Motion::Static, Motion::Keyframed]);
    assert_eq!(tallies, ["static_base_fixed"]);
    let mut mstt = model();
    prepare::fix_static_base(Some(*b"MSTT"), &mut mstt, &mut tallies);
    assert_eq!(mstt[0].motion, Motion::Dynamic);
    assert_eq!(tallies.len(), 1);
}

/// A clutter skeleton's collision lies in the pose its `XRGD` stores, as its drawing does
/// (playtest list 5 item 71): a torso standing 69 up with a skull 45 above it, laid on its
/// back 10 above the floor, puts the skull 45 along the floor, not over the torso.
#[test]
fn stored_pose_lays_the_collision_down() {
    use fnv_collide::nif::{NodeTree, Similarity};
    use fnv_world::xrgd::Bone;
    let at = |t: [f32; 3]| Similarity { iso: Iso { translation: t, rotation: Iso::IDENTITY.rotation }, scale: 1.0 };
    // Root 0 → torso 1 → neck 2 (no body) → skull 3.
    let tree = NodeTree::from_nodes([(0, None, Similarity::IDENTITY), (1, Some(0), at([0.0, 0.0, 69.0])), (2, Some(1), at([0.0, 0.0, 40.0])), (3, Some(2), at([0.0, 0.0, 5.0]))]);
    let body = |block: usize, z: f32| CollisionObject { node: Some(format!("n{block}")), node_block: block, node_transform: Iso { translation: [0.0, 0.0, z], rotation: Iso::IDENTITY.rotation }, ..object(4.0, Motion::Dynamic) };
    let standing = vec![body(1, 69.0), body(3, 114.0)];
    // The torso turned a quarter about y (z onto x), the skull unturned under the neck.
    let bones = [Bone { position: [0.0, 0.0, 10.0], angles: [0.0, -std::f32::consts::FRAC_PI_2, 0.0] }, Bone { position: [0.0, 0.0, 5.0], angles: [0.0; 3] }];
    let mut objects = standing.clone();
    assert!(prepare::pose_stored(&tree, &bones, &mut objects));
    let p = |o: &CollisionObject| o.node_transform.translation;
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
    assert!(near(p(&objects[0]), [0.0, 0.0, 10.0]), "{:?}", p(&objects[0]));
    assert!(near(p(&objects[1]), [45.0, 0.0, 10.0]), "{:?}", p(&objects[1]));
    assert!(near(objects[1].node_transform.apply([0.0, 0.0, 1.0]), [46.0, 0.0, 10.0]), "the skull lies turned with the torso");
    // Entries that do not fit leave the file's pose.
    let mut objects = standing.clone();
    assert!(!prepare::pose_stored(&tree, &bones[..1], &mut objects));
    assert_eq!(objects, standing);
}

/// A small creature's character is its own size, a person's the player's (playtest list
/// 7 item 35: the mantis nymphs, drawn about 30 units tall, had the player's capsule).
#[test]
fn small_actors_get_characters_of_their_size() {
    let person = super::npc_params(Some(127.0));
    assert_eq!((person.radius, person.height), (22.0, fnv_phys::CharacterParams::default().height));
    assert_eq!(super::npc_params(None).radius, 22.0);
    let nymph = super::npc_params(Some(30.0));
    assert_eq!((nymph.radius, nymph.height), (crate::blocking::MIN_RADIUS, 30.0));
    let dog = super::npc_params(Some(80.0));
    assert!((dog.radius - 13.6).abs() < 1e-3 && dog.height == 80.0, "{dog:?}");
    assert_eq!(super::npc_params(Some(3.0)).height, super::NPC_MIN_HEIGHT);
}

/// The gate of [`gate_cell`]: placed at x 1000 in square (0, 0); its "Forward" (played
/// once, 1 s) slides its keyframed panel 300 units along x.
const GATE: u32 = 0x0200_0002;
const GATE_X: f32 = 1000.0;

fn gate_anim() -> fnv_collide::anim::ModelAnim {
    use fnv_collide::nif::Similarity;
    use fnv_nif::anim::{Channel, Key, KeyKind, Keys, TransformTrack};
    let mut anim = fnv_collide::anim::ModelAnim::from_nodes([(0, Some("Root".to_string()), None, Similarity::IDENTITY), (1, Some("Panel".to_string()), Some(0), Similarity::IDENTITY)]);
    let key = |time: f32, x: f32| Key { time, value: [x, 0.0, 0.0], forward: [0.0; 3], backward: [0.0; 3], tbc: [0.0; 3] };
    let slide = TransformTrack {
        translation: Channel::Keys(Keys { kind: KeyKind::Linear, keys: vec![key(0.0, 0.0), key(1.0, 300.0)] }),
        rotation: Channel::Static(None),
        euler: None,
        scale: Channel::Static(None),
    };
    anim.add_sequence("Forward", 1.0, 2, vec![(1, slide)]);
    anim
}

/// [`fake_cell`], with square (0, 0) also holding [`GATE`]: an activator (as
/// `prepare::reference` finds it) whose one collision object, a keyframed 100-unit wall,
/// hangs from the node its sequence moves.
fn gate_cell(job: &Job) -> Prepared {
    let mut p = fake_cell(job);
    if job.cell == PhysCell::Exterior(0, 0) {
        let pl = Placement {
            form_id: GATE,
            sig: *b"REFR",
            flags: 0,
            base: None,
            position: [GATE_X, 3000.0, 100.0],
            rotation: [0.0; 3],
            scale: 1.0,
            enable_parent: None,
            open_by_default: false,
            teleport: false,
            emittance: None,
            imposter: false,
            primitive: false,
        };
        let obj = CollisionObject { node: Some("Panel".into()), node_block: 1, shape: Shape::Box { half_extents: [50.0, 10.0, 100.0] }.into(), ..object(0.0, Motion::Keyframed) };
        let prepared = PreparedRef::new(&PhysParams::default(), &RefPose::from_placement(&pl), &[obj]);
        let animated = Some(prepare::AnimatedBodies { anim: Arc::new(gate_anim()), objects: vec![(0, 1, Iso::IDENTITY)] });
        p.refs.push(Entry { form_id: pl.form_id, placement: Some(pl), shapes: Some(RefShapes { tallies: vec!["fake"], water: Vec::new(), prepared: Some(prepared), dynamic_local: None, door: None, animated, drawn: None, model: None }) });
    }
    p
}

/// Issue #1396: an animated object's collision follows the group its drawn model plays
/// (the TTW Megaton gate stayed solid where it stood closed), stays at the end of a group
/// played once, and is posed there from the last group asked when nothing draws it (after
/// a load).
#[test]
fn an_animated_objects_collision_follows_its_group() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = PhysState { fake: Some(gate_cell), loader: loader::Loader::manual(), ..Default::default() };
    p.load(&ctx, PhysCell::Exterior(0, 0));
    assert!(p.animated.contains_key(&GATE));
    let x = |p: &PhysState| p.world.ref_bodies(GATE)[0].translation[0];
    // A ray along y, 20 units off the ground, through where the panel stands closed.
    let blocks = |p: &mut PhysState, at: f32| p.world.queries().ray([at, 2900.0, 120.0], [0.0, 1.0, 0.0], 200.0, &Filter { no_terrain: true, ..Default::default() }).is_some_and(|h| matches!(h.owner, Owner::Ref { form_id: GATE, .. }));
    assert!((x(&p) - GATE_X).abs() < 1e-3);
    assert!(blocks(&mut p, GATE_X));
    // Nothing plays: the bind pose stays.
    p.pose_animated(None, |_| None);
    assert!((x(&p) - GATE_X).abs() < 1e-3);
    // Halfway through, as drawn.
    let mut poses = ObjectPoses::default();
    poses.playing.insert(GATE, ("Forward".into(), 0.5));
    p.pose_animated(Some(&poses), |_| None);
    assert!((x(&p) - (GATE_X + 150.0)).abs() < 1e-2, "{}", x(&p));
    // At its end: out of the way, solid where it is drawn open.
    poses.playing.insert(GATE, ("Forward".into(), 1.0));
    p.pose_animated(Some(&poses), |_| None);
    assert!((x(&p) - (GATE_X + 300.0)).abs() < 1e-2, "{}", x(&p));
    assert!(!blocks(&mut p, GATE_X), "the way through is open");
    assert!(blocks(&mut p, GATE_X + 300.0), "the open panel is solid");
    // Reloaded (the cell and its collision built again, nothing drawn yet): posed at the
    // end of the last group asked, which the saved game keeps.
    p.unload(PhysCell::Exterior(0, 0));
    assert!(!p.animated.contains_key(&GATE));
    p.load(&ctx, PhysCell::Exterior(0, 0));
    assert!((x(&p) - GATE_X).abs() < 1e-3, "built closed");
    p.pose_animated(None, |r| (r == GATE).then_some("forward"));
    assert!((x(&p) - (GATE_X + 300.0)).abs() < 1e-2, "{}", x(&p));
    assert!(!blocks(&mut p, GATE_X));
    // A sequence the model does not have, or none: the bind pose again.
    p.pose_animated(None, |_| Some("Backward"));
    assert!((x(&p) - GATE_X).abs() < 1e-3);
}

#[test]
fn the_pose_of_a_group_not_drawn_is_its_end_or_its_loops_start() {
    let mut anim = gate_anim();
    anim.add_sequence("Idle", 2.0, 0, Vec::new());
    assert_eq!(super::group_pose(&anim, None, Some("Forward")), (Some(0), 1.0));
    assert_eq!(super::group_pose(&anim, None, Some("idle")), (Some(1), 0.0));
    assert_eq!(super::group_pose(&anim, Some(&("Idle".into(), 0.7)), Some("Forward")), (Some(1), 0.7));
    // What is drawn but not in the collision's model: the last group asked.
    assert_eq!(super::group_pose(&anim, Some(&("Other".into(), 0.7)), Some("Forward")), (Some(0), 1.0));
    assert_eq!(super::group_pose(&anim, None, None), (None, 0.0));
}

#[test]
fn a_place_near_loaded_collision_only_is_refused_and_a_character_goes_exactly_where_told() {
    let (d, sim) = (play_data(), sim());
    let ctx = LoadCtx { d: &d, sim: &sim, data: None };
    let mut p = PhysState { fake: Some(fake_cell), loader: loader::Loader::manual(), ..Default::default() };
    let start = centre(0, 0);
    p.stream(&ctx, start, CELLS_PER_FRAME);
    // The loader has not delivered: only the cell underfoot is in.
    assert!(p.ready_at(start));
    assert!(p.ready_near(start, 256.0), "the middle of a square is far from its neighbours");
    assert!(!p.ready_near([CELL_SIZE - 10.0, start[1], 50.0], 256.0), "the square next door is not in");
    assert!(p.add_npc(7, [start[0] + 200.0, start[1] + 200.0, 200.0]));
    assert_eq!(p.npc_spot(7, [CELL_SIZE - 10.0, start[1], 50.0]), Err(NpcSpotRefused::NotLoaded));
    assert_eq!(p.npc_spot(8, start), Err(NpcSpotRefused::NotLoaded), "no character");
    // Everything in: a spot on the sloping terrain, and the character put on it to the unit.
    p.loader.run_manual(false);
    for _ in 0..8 {
        p.stream(&ctx, start, CELLS_PER_FRAME);
    }
    assert!(p.ready_near([CELL_SIZE - 10.0, start[1], 50.0], 256.0));
    let ground = p.terrain_z(start[0] + 200.0, start[1] - 400.0).expect("terrain");
    let at = [start[0] + 200.0, start[1] - 400.0, ground];
    let feet = p.npc_spot(7, at).expect("open ground");
    assert!((feet[2] - ground).abs() < 6.0, "{feet:?} on {ground}");
    p.npcs.get_mut(&7).unwrap().velocity = [100.0, 0.0];
    assert!(p.place_npc(7, feet));
    let b = &p.npcs[&7];
    assert_eq!((b.ch.feet, b.prev_feet, b.velocity), (feet, feet, [0.0, 0.0]));
    assert!(!p.place_npc(8, feet));
    // Too far over the ground for the level asked for.
    assert_eq!(p.npc_spot(7, [at[0], at[1], ground + 1000.0]), Err(NpcSpotRefused::NoSupport));
    // The player takes up room.
    p.place_player(feet, 48.0, 400.0);
    assert_eq!(p.npc_spot(7, [at[0] + 5.0, at[1], at[2]]), Err(NpcSpotRefused::NoRoom));
}
