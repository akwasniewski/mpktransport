use std::{env, path::Path};

use anyhow::{Context, Result};
use mpktransport::{app::App, graph::Graph};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path/to/gtfs_directory>", args[0]);
        eprintln!("Generate data files first with: cargo run --release --bin create_footpaths data/krakow.osm.pbf data/GTFS_KRK_T");
        std::process::exit(1);
    }

    let dir = Path::new(&args[1]);
    if !dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", dir.display());
    }

    println!("Loading GTFS from: {}", dir.display());
    let graph = Graph::load(dir).context("Failed to load GTFS")?;
    println!("  stops loaded: {}", graph.stops.len());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GTFS Viewer")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GTFS Viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(graph, cc.egui_ctx.clone())))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
