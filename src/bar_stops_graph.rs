use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

use crate::graph::Graph;
use crate::utils::Secs;

const WALK_SPEED_MPS: f64 = 2.2;

#[derive(Debug, Clone)]
pub struct Bar {
    pub place_id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub vicinity: String,
}

#[derive(Debug, Deserialize)]
struct RawRow {
    bar_place_id: String,
    bar_name: String,
    #[serde(default)]
    bar_vicinity: String,
    bar_lat: f64,
    bar_lon: f64,
    stop_id: String,
    #[serde(default)]
    straight_line_m: f64,
    #[serde(default)]
    walk_duration_s: Option<Secs>,
}

pub struct BarsStops {
    pub bars: Vec<Bar>,
    pub footpaths: HashMap<String, Vec<(usize, Secs)>>,
}

impl BarsStops {
    pub fn empty() -> Self {
        BarsStops { bars: Vec::new(), footpaths: HashMap::new() }
    }

    pub fn load(path_to_csv: &Path, graph: &Graph) -> Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .from_path(path_to_csv)
            .with_context(|| format!("opening {}", path_to_csv.display()))?;

        let mut bars: Vec<Bar> = Vec::new();
        let mut footpaths: HashMap<String, Vec<(usize, Secs)>> = HashMap::new();

        for row in rdr.deserialize::<RawRow>() {
            let r = match row {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Warning: skipping bars_stops row – {e}");
                    continue;
                }
            };

            let Some(&stop_idx) = graph.stops_by_id.get(&r.stop_id) else {
                eprintln!(
                    "Warning: unknown stop_id {} for bar {}, skipping",
                    r.stop_id, r.bar_name
                );
                continue;
            };

            let walk = r
                .walk_duration_s
                .unwrap_or_else(|| (r.straight_line_m / WALK_SPEED_MPS).round() as Secs);

            if !footpaths.contains_key(&r.bar_place_id) {
                bars.push(Bar {
                    place_id: r.bar_place_id.clone(),
                    name: r.bar_name,
                    lat: r.bar_lat,
                    lon: r.bar_lon,
                    vicinity: r.bar_vicinity,
                });
            }

            footpaths
                .entry(r.bar_place_id)
                .or_default()
                .push((stop_idx, walk));
        }

        if bars.is_empty() {
            eprintln!("Warning: no bars loaded from {}", path_to_csv.display());
        }
        Ok(BarsStops { bars, footpaths })
    }
}
