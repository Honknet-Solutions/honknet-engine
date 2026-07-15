use std::time::Instant;

use honknet_core::{EntityId, SpatialHash, World};

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
}

fn main() {
    let entities = env_usize("HONKNET_BENCH_ENTITIES", 100_000);
    let iterations = env_usize("HONKNET_BENCH_ITERATIONS", 300);
    println!("Honknet core benchmark: entities={entities} iterations={iterations}");

    let mut world = World::new();
    let spawn_started = Instant::now();
    for index in 0..entities {
        let entity = world.spawn();
        world
            .add_component(
                entity,
                Position {
                    x: (index % 1_000) as f32,
                    y: (index / 1_000) as f32,
                },
            )
            .expect("fresh entity");
        if index % 2 == 0 {
            world
                .add_component(entity, Velocity { x: 0.1, y: -0.1 })
                .expect("fresh entity");
        }
    }
    let spawn_elapsed = spawn_started.elapsed();

    let simulation_started = Instant::now();
    let mut updates = 0_u64;
    let mut moving = Vec::new();
    for _ in 0..iterations {
        world.query_ids2_into::<Position, Velocity>(&mut moving);
        updates += moving.len() as u64;
        for &entity in &moving {
            let velocity = *world
                .get_component::<Velocity>(entity)
                .expect("query guarantees velocity");
            if let Some(position) = world.get_component_mut::<Position>(entity) {
                position.x += velocity.x;
                position.y += velocity.y;
            }
        }
    }
    let simulation_elapsed = simulation_started.elapsed();

    let spatial_started = Instant::now();
    let mut spatial = SpatialHash::new(8.0);
    for entity in world.query_ids::<Position>() {
        let position = world
            .get_component::<Position>(entity)
            .expect("query guarantees position");
        spatial.insert_circle(entity, 1, 0, position.x, position.y, 0.35);
    }
    let spatial_build_elapsed = spatial_started.elapsed();

    let query_started = Instant::now();
    let mut candidates = 0_usize;
    for index in 0..10_000 {
        candidates += spatial
            .query_circle(1, 0, (index % 1_000) as f32, (index / 1_000) as f32, 32.0)
            .len();
    }
    let query_elapsed = query_started.elapsed();

    println!("spawn_ms={:.3}", spawn_elapsed.as_secs_f64() * 1_000.0);
    println!(
        "simulation_ms={:.3} updates={} updates_per_second={:.0}",
        simulation_elapsed.as_secs_f64() * 1_000.0,
        updates,
        updates as f64 / simulation_elapsed.as_secs_f64().max(f64::EPSILON),
    );
    println!(
        "spatial_build_ms={:.3} indexed={}",
        spatial_build_elapsed.as_secs_f64() * 1_000.0,
        spatial.entity_count(),
    );
    println!(
        "spatial_query_ms={:.3} queries=10000 candidates={candidates}",
        query_elapsed.as_secs_f64() * 1_000.0,
    );

    // Keep EntityId referenced directly so public benchmark builds also verify
    // that the engine's public id API remains usable by downstream crates.
    let _ = EntityId::new(1);
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}
