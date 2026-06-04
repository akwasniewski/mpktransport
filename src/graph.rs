use anyhow::{Context, Result};
use serde::Deserialize;
use std::{env, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    #[serde(default)]
    pub stop_code: String,
    #[serde(default)]
    pub stop_name: String,
    #[serde(default)]
    pub stop_desc: String,
    #[serde(default)]
    pub stop_lat: Option<f64>,
    #[serde(default)]
    pub stop_lon: Option<f64>,
    #[serde(default)]
    pub zone_id: String,
    #[serde(default)]
    pub stop_url: String,
    #[serde(default)]
    pub location_type: Option<u8>,
    #[serde(default)]
    pub parent_station: String,
    #[serde(default)]
    pub stop_timezone: String,
    #[serde(default)]
    pub wheelchair_boarding: Option<u8>,
    #[serde(default)]
    pub platform_code: String,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub stops: Vec<Stop>,
    pub source_dir: String,
}

impl Graph {
    pub fn load(dir: &Path) -> Result<Self> {
        let stops = load_stops(&dir.join("stops.txt"))?;
        Ok(Self {
            stops,
            source_dir: dir.to_string_lossy().into_owned(),
        })
    }

    /// Average lat/lon of all stops that have coordinates.
    pub fn centre(&self) -> Option<(f64, f64)> {
        let coords: Vec<(f64, f64)> = self
            .stops
            .iter()
            .filter_map(|s| Some((s.stop_lat?, s.stop_lon?)))
            .collect();
        if coords.is_empty() {
            return None;
        }
        let n = coords.len() as f64;
        Some((
            coords.iter().map(|c| c.0).sum::<f64>() / n,
            coords.iter().map(|c| c.1).sum::<f64>() / n,
        ))
    }
}


fn load_stops(path: &Path) -> Result<Vec<Stop>> {
    if !path.exists() {
        eprintln!("Warning: {} not found", path.display());
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;

    let mut stops = Vec::new();
    for result in rdr.deserialize::<Stop>() {
        match result {
            Ok(s) => stops.push(s),
            Err(e) => eprintln!("Warning: skipping row – {e}"),
        }
    }
    Ok(stops)
}
