use anyhow::{Context, Result};
use egui::ahash::HashMap;
use serde::Deserialize;
use std::{path::Path};

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

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub route_id: String,
    #[serde(default)]
    pub agency_id: String,
    #[serde(default)]
    pub route_short_name: String,
    #[serde(default)]
    pub route_long_name: String,
    #[serde(default)]
    pub route_type: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Trip {
    pub route_id: String,
    pub service_id: String,
    pub trip_id: String,
    #[serde(default)]
    pub trip_headsign: String,
    #[serde(default)]
    pub direction_id: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopTime {
    pub trip_id: String,
    pub arrival_time: String,
    pub departure_time: String,
    pub stop_id: String,
    pub stop_sequence: u32,
    #[serde(default)]
    pub stop_headsign: String,
    #[serde(default)]
    pub pickup_type: Option<u16>,
    #[serde(default)]
    pub drop_off_type: Option<u16>,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub stops: Vec<Stop>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    pub stop_times: Vec<StopTime>,
    pub source_dir: String,

    // indexes
    pub stops_by_id: HashMap<String, usize>,
    pub routes_by_id: HashMap<String, usize>,
    pub trips_by_id: HashMap<String, usize>,
    pub trips_by_route: HashMap<String, Vec<usize>>,

    pub stop_times_by_trip: HashMap<String, Vec<StopTime>>,
    pub stop_times_by_stop: HashMap<String, Vec<usize>>,

    /// (route_id, direction_id) → stop indices (into `stops`), in stop_sequence order
    pub stops_by_route: HashMap<(String, usize), Vec<usize>>,

    /// (trip_id, stop_id) → (arrival_secs, departure_secs), pre-parsed for O(1) lookup
    pub times_at: HashMap<String, HashMap<String, (u32, u32)>>,
    }

impl Graph {
    pub fn load(dir: &Path) -> Result<Self> {
        let stops = load_csv(&dir.join("stops.txt"))?;
        let routes = load_csv(&dir.join("routes.txt"))?;
        let trips = load_csv(&dir.join("trips.txt"))?;
        let stop_times = load_csv(&dir.join("stop_times.txt"))?;

        let mut graph = Self {
            stops,
            routes,
            trips,
            stop_times,
            source_dir: dir.to_string_lossy().into_owned(),
            ..Default::default()
        };

        graph.build_indexes();

        Ok(graph)
    }

    fn build_indexes(&mut self) {
        self.stops_by_id = self
            .stops
            .iter()
            .enumerate()
            .map(|(i, s)| (s.stop_id.clone(), i))
            .collect();

        self.routes_by_id = self
            .routes
            .iter()
            .enumerate()
            .map(|(i, r)| (r.route_id.clone(), i))
            .collect();

        self.trips_by_id = self
            .trips
            .iter()
            .enumerate()
            .map(|(i, t)| (t.trip_id.clone(), i))
            .collect();

        self.trips_by_route.clear();
        for (i, t) in self.trips.iter().enumerate() {
            self.trips_by_route
                .entry(t.route_id.clone())
                .or_default()
                .push(i);
        }

        self.stop_times_by_trip.clear();
        self.stop_times_by_stop.clear();

        for (i, st) in self.stop_times.iter().enumerate() {
            self.stop_times_by_trip
                .entry(st.trip_id.clone())
                .or_default()
                .push(st.clone());

            self.stop_times_by_stop
                .entry(st.stop_id.clone())
                .or_default()
                .push(i);
        }

        // Pre-sort by stop_sequence so callers get ordered stops for free.
        for stop_times in self.stop_times_by_trip.values_mut() {
            stop_times.sort_by_key(|st| st.stop_sequence);
        }

        // Build stops_by_trip, stops_by_route and times_at from the now-sorted stop_times_by_trip.
        self.stops_by_route.clear();
        self.times_at.clear();
        for (trip_id, sts) in &self.stop_times_by_trip {
            let stop_idxs: Vec<usize> = sts
                .iter()
                .filter_map(|st| self.stops_by_id.get(&st.stop_id).copied())
                .collect();

            if let Some(&trip_idx) = self.trips_by_id.get(trip_id) {
                let trip = &self.trips[trip_idx];
                let direction = trip.direction_id.unwrap_or(0) as usize;
                let key = (trip.route_id.clone(), direction);
                self.stops_by_route.entry(key)
                    .and_modify(|existing| {
                        if stop_idxs.len() > existing.len() {
                            *existing = stop_idxs.clone();
                        }
                    })
                    .or_insert(stop_idxs);
            }

            let by_stop = sts
                .iter()
                .filter_map(|st| {
                    let arr = parse_time(&st.arrival_time)?;
                    let dep = parse_time(&st.departure_time)?;
                    Some((st.stop_id.clone(), (arr, dep)))
                })
                .collect();
            self.times_at.insert(trip_id.clone(), by_stop);
        }
    }

    pub fn arrival_at(&self, trip_id: &str, stop_id: &str) -> Option<u32> {
        self.times_at.get(trip_id)?.get(stop_id).map(|&(arr, _)| arr)
    }

    pub fn departure_at(&self, trip_id: &str, stop_id: &str) -> Option<u32> {
        self.times_at.get(trip_id)?.get(stop_id).map(|&(_, dep)| dep)
    }

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

/// Parse a GTFS time string ("HH:MM:SS", hours may exceed 23) into seconds since midnight.
/// Returns `None` if the string is malformed.
pub fn parse_time(s: &str) -> Option<u32> {
    let mut parts = s.splitn(3, ':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let sec: u32 = parts.next()?.parse().ok()?;
    Some(h * 3600 + m * 60 + sec)
}

fn load_csv<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        eprintln!("Warning: {} not found", path.display());
        return Ok(vec![]);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;

    let mut out = Vec::new();

    for row in rdr.deserialize::<T>() {
        match row {
            Ok(v) => out.push(v),
            Err(e) => eprintln!("Warning: skipping row – {e}"),
        }
    }

    Ok(out)
}
