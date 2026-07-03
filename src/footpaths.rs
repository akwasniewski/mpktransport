use crate::utils::Secs;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::{Path, PathBuf}};

pub type Footpaths = HashMap<usize, Vec<(usize, Secs)>>;

#[derive(Debug, Deserialize)]
struct FootpathRow {
    from_stop: usize,
    to_stop: usize,
    time_secs: Secs,
}

pub fn default_path(gtfs_dir: &Path) -> PathBuf {
    gtfs_dir
        .parent()
        .unwrap_or(gtfs_dir)
        .join("footpaths.csv")
}

pub fn load(path: &Path) -> Result<Footpaths> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("cannot open {}", path.display()))?;

    let mut footpaths: Footpaths = HashMap::new();
    for row in rdr.deserialize::<FootpathRow>() {
        let row = row.with_context(|| format!("invalid row in {}", path.display()))?;
        footpaths
            .entry(row.from_stop)
            .or_default()
            .push((row.to_stop, row.time_secs));
    }

    println!("Loaded {} footpath sources from {}", footpaths.len(), path.display());

    Ok(footpaths)
}
