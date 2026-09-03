use grove_core::{goal_fragility, Kind};
use grove_desktop_lib::views::{cone, load_state};
use std::time::Instant;

fn main() {
    let mut root = ".".to_string();
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--root=") {
            root = v.to_string();
        }
    }
    let root = grove_core::abspath(&root);
    let st = load_state(&root).expect("state loads");
    let seed = st
        .nodes
        .values()
        .filter(|n| n.kind == Kind::W && !n.archived)
        .max_by_key(|n| n.lines("goals").len())
        .expect("a work item exists");
    let goals = seed.lines("goals").len();
    println!(
        "seed {} with {} goal(s), lock nodes {}",
        seed.id,
        goals,
        st.nodes.len()
    );
    let mut best = u128::MAX;
    let mut worst = 0u128;
    const BUILDS: usize = 20;
    for _ in 0..BUILDS {
        let t0 = Instant::now();
        let m = cone::model(&st, &seed.id, 4, 50);
        let fragility = goal_fragility(&st, seed);
        let elapsed = t0.elapsed().as_millis();
        assert!(m.get("cone").is_some(), "model builds a cone");
        assert!(!fragility.is_empty() || goals == 0, "fragility covers goals");
        println!("build: {} ms (connectivity {:?})", elapsed, fragility);
        best = best.min(elapsed);
        worst = worst.max(elapsed);
    }
    println!(
        "cone_timing seed={} goals={} builds={} best_ms={} worst_ms={}",
        seed.id, goals, BUILDS, best, worst
    );
}
