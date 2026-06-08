use anyhow::{Context, Result};
use std::path::Path;

use mpktransport::graph::Graph;
use mpktransport::raptor::{Raptor, Secs};

pub fn fmt_time(secs: Secs) -> String {
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}

fn main() -> Result<()> {
    let graph = Graph::load(Path::new("data/GTFS_KRK_T"))
        .context("failed to load GTFS")?;

    println!("Loaded {} stops, {} routes", graph.stops.len(), graph.routes.len());

    let departure: Secs = 8 * 3600; // 08:00
    println!("\nDeparture: {}\n", fmt_time(departure));

    let from = "stop_346_269029";
    let to = "stop_321_115419";
    // let to = "stop_191_7719";

    let mut raptor = Raptor::new(&graph);
    match raptor.query(&from, &to, departure) {
        Some(j) => {
            println!("Arrive: {}", fmt_time(j.arrival));
            println!("Path ({} stops):", j.legs.len());
            for (i, stop) in j.legs.iter().enumerate() {
                println!("  {:>2}. {} at {}", i + 1, stop.1, fmt_time(stop.0));
            }
        }
        None => println!("No route found."),
    }

    Ok(())
}
