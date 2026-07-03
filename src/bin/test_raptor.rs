use anyhow::{Context, Result};
use mpktransport::utils::{fmt_time, Secs};
use std::path::Path;

use mpktransport::footpaths;
use mpktransport::graph::Graph;
use mpktransport::raptor::Raptor;

fn main() -> Result<()> {
    let gtfs_dir = Path::new("data/GTFS_KRK_T");
    let graph = Graph::load(gtfs_dir)
        .context("failed to load GTFS")?;
    let footpaths = footpaths::load(&footpaths::default_path(gtfs_dir))
        .context("failed to load footpaths")?;

    println!("Loaded {} stops, {} routes", graph.stops.len(), graph.routes.len());

    let departure: Secs = 8 * 3600; // 08:00
    println!("\nDeparture: {}\n", fmt_time(departure));

    let mut raptor = Raptor::new(&graph, &footpaths);
    match raptor.query(0, 230, departure) {
        Some(j) => {
            println!("Arrive: {}", fmt_time(j.arrival));
            println!("Path ({} stops):", j.legs.len());
            for (i, stop) in j.legs.iter().enumerate() {
                println!("  {:>2}. {} at {}", i + 1, stop.stop_name, fmt_time(stop.time));
            }
        }
        None => println!("No route found."),
    }

    Ok(())
}
