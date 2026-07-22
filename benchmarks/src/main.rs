use honknet_ecs::{Component, World};
use std::time::Instant;
struct Position(f32, f32);
impl Component for Position {}

fn main() {
    let n = std::env::var("HONKNET_BENCH_ENTITIES")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(100_000);
    let mut w = World::default();
    let t = Instant::now();
    for i in 0..n {
        let e = w.spawn();
        w.insert(e, Position(i as f32, 0.)).unwrap();
    }
    let spawn = t.elapsed();
    let t = Instant::now();
    for e in w.query::<Position>() {
        let p = w.get_mut::<Position>(e).unwrap();
        p.0 += 1.;
        p.1 += 1.;
    }
    println!(
        "entities={n} spawn_ms={} update_ms={}",
        spawn.as_secs_f64() * 1000.,
        t.elapsed().as_secs_f64() * 1000.
    );
}
