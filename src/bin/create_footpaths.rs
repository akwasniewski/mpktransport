use anyhow::{Context, Result};
use mpktransport::{
    graph::{Graph, Stop},
    utils::Secs,
};
use serde::Serialize;
use std::{
    env,
    path::{Path, PathBuf},
};
use transport::{
    algo::astar,
    graph::Graph as RoadGraph,
    graph_building::{parse_osm, ParseConfig},
};

#[derive(Debug, Serialize)]
struct FootpathRow {
    from_stop: usize,
    to_stop: usize,
    time_secs: Secs,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() > 6 {
        eprintln!(
            "Usage: {} <path/to/map.osm.pbf> <path/to/gtfs_directory> [prefix] [data_dir] [footpaths_csv]",
            args[0]
        );
        eprintln!("Defaults: prefix=krakow, data_dir=<gtfs_directory>, footpaths_csv=<gtfs_directory>/footpaths.csv");
        eprintln!("Example: {} data/krakow.osm.pbf data/GTFS_KRK_T", args[0]);
        std::process::exit(1);
    }

    let pbf_path = Path::new(&args[1]);
    if !pbf_path.is_file() {
        anyhow::bail!("'{}' is not a file", pbf_path.display());
    }

    let gtfs_dir = Path::new(&args[2]);
    if !gtfs_dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", gtfs_dir.display());
    }

    let prefix = args.get(3).map(String::as_str).unwrap_or("krakow");
    let data_dir = args
        .get(4)
        .map(PathBuf::from)
        .unwrap_or_else(|| gtfs_dir.to_path_buf());
    let footpaths_path = args
        .get(5)
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("footpaths.csv"));

    let result = parse_osm(pbf_path, &data_dir, prefix, ParseConfig::default()).map_err(|err| {
        anyhow::anyhow!(
            "failed to create road graph from '{}': {err}",
            pbf_path.display()
        )
    })?;

    let snap_path = data_dir.join(format!("{prefix}_snap.txt"));
    let coords_path = data_dir.join(format!("{prefix}_coords.txt"));

    println!("Created road graph:");
    println!("  nodes: {}", result.node_count);
    println!("  edges: {}", result.edge_count);
    println!("  snap: {}", snap_path.display());
    println!("  coords: {}", coords_path.display());

    println!("Loading GTFS from: {}", gtfs_dir.display());
    let graph = Graph::load(gtfs_dir).context("failed to load GTFS")?;

    println!("Loading road graph...");
    let road_graph = RoadGraph::from_files(&snap_path.to_string_lossy(), &coords_path.to_string_lossy());

    let footpaths = compute_footpaths(&graph, &road_graph);
    write_footpaths(&footpaths_path, &footpaths)?;

    println!(
        "Wrote {} footpaths to {}",
        footpaths.len(),
        footpaths_path.display()
    );

    Ok(())
}

fn find_closest_road_node(stop: &Stop, road_graph: &RoadGraph) -> Option<usize> {
    let stop_lat = stop.stop_lat?;
    let stop_lon = stop.stop_lon?;
    let stop_lat_rad = stop_lat.to_radians();

    road_graph
        .vertices
        .iter()
        .enumerate()
        .filter_map(|(idx, vertex)| {
            let (lat, lon) = vertex.coords;
            if !lat.is_finite() || !lon.is_finite() {
                return None;
            }

            let lat = lat as f64;
            let lon = lon as f64;
            let d_lat = (lat - stop_lat).to_radians();
            let d_lon = (lon - stop_lon).to_radians() * stop_lat_rad.cos();

            Some((idx, d_lat * d_lat + d_lon * d_lon))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(idx, _)| idx)
}

fn compute_footpaths(graph: &Graph, road_graph: &RoadGraph) -> Vec<FootpathRow> {
    println!("Computing footpaths...");

    let road_stops: Vec<(usize, usize)> = graph
        .stops
        .iter()
        .filter_map(|stop| {
            find_closest_road_node(stop, road_graph).map(|road_node| (stop.idx, road_node))
        })
        .collect();

    println!("Matched {} stops to road graph nodes", road_stops.len());

    const WALKING_SPEED_KM_H: f32 = 5.0;
    const MAX_WALKING_TIME_SECS: Secs = 30 * 60;
    let walking_speed_m_s = WALKING_SPEED_KM_H / 3.6;

    let mut footpaths = Vec::new();
    for &(from_stop, from_road_node) in &road_stops {
        for &(to_stop, to_road_node) in &road_stops {
            if from_stop == to_stop || from_road_node == to_road_node {
                footpaths.push(FootpathRow { 
                    from_stop,
                    to_stop,
                    time_secs: 30, // minimum interchange 30 seconds
                });
                continue;
            }

            if let Some(distance_m) =
                astar(road_graph, from_road_node as u32, to_road_node as u32).distance
            {
                let time_secs = (distance_m / walking_speed_m_s).ceil() as Secs;
                if time_secs <= MAX_WALKING_TIME_SECS {
                    footpaths.push(FootpathRow {
                        from_stop,
                        to_stop,
                        time_secs,
                    });
                }
            }
        }
    }

    footpaths
}

fn write_footpaths(path: &Path, footpaths: &[FootpathRow]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }

    let mut writer =
        csv::Writer::from_path(path).with_context(|| format!("cannot create {}", path.display()))?;
    for footpath in footpaths {
        writer.serialize(footpath)?;
    }
    writer.flush()?;

    Ok(())
}
