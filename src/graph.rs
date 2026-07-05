use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::{HashMap, HashSet}, path::Path};

use crate::utils::Secs;

#[derive(Debug, Deserialize)]
struct RawShapePoint {
    shape_id: String,
    shape_pt_lat: f64,
    shape_pt_lon: f64,
    shape_pt_sequence: u32,
    #[serde(default)]
    shape_dist_traveled: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawStop {
    stop_id: String,
    #[serde(default)]
    stop_code: String,
    #[serde(default)]
    stop_name: String,
    #[serde(default)]
    stop_desc: String,
    #[serde(default)]
    stop_lat: Option<f64>,
    #[serde(default)]
    stop_lon: Option<f64>,
    #[serde(default)]
    zone_id: String,
    #[serde(default)]
    stop_url: String,
    #[serde(default)]
    location_type: Option<u8>,
    #[serde(default)]
    parent_station: String,
    #[serde(default)]
    stop_timezone: String,
    #[serde(default)]
    wheelchair_boarding: Option<u8>,
    #[serde(default)]
    platform_code: String,
}

#[derive(Debug, Deserialize)]
struct RawRoute {
    route_id: String,
    #[serde(default)]
    agency_id: String,
    #[serde(default)]
    route_short_name: String,
    #[serde(default)]
    route_long_name: String,
    #[serde(default)]
    route_type: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct RawTrip {
    route_id: String,
    service_id: String,
    trip_id: String,
    #[serde(default)]
    trip_headsign: String,
    #[serde(default)]
    direction_id: Option<u16>,
    #[serde(default)]
    shape_id: String,
}

#[derive(Debug, Deserialize)]
struct RawStopTime {
    trip_id: String,
    arrival_time: String,
    departure_time: String,
    stop_id: String,
    stop_sequence: u32,
    #[serde(default)]
    stop_headsign: String,
    #[serde(default)]
    pickup_type: Option<u16>,
    #[serde(default)]
    drop_off_type: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct Stop {
    pub idx: usize,
    pub stop_code: String,
    pub name: String,
    pub stop_desc: String,
    pub stop_lat: Option<f64>,
    pub stop_lon: Option<f64>,
    pub zone_id: String,
    pub stop_url: String,
    pub location_type: Option<u8>,
    pub station: usize,
    pub foothpaths: Vec<(usize, Secs)>,
    pub stop_timezone: String,
    pub wheelchair_boarding: Option<u8>,
    pub platform_code: String,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub idx: usize,
    pub agency_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
    pub route_type: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct Station {
    pub idx: usize,
    pub name: String,
    pub stops: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub idx: usize,
    pub route_idx: usize,
    pub service_idx: usize,
    pub trip_headsign: String,
    pub direction_id: u16,
    pub shape_idx: Option<usize>,
}

/// A single stop-visit within a trip, with times pre-parsed to seconds.
#[derive(Debug, Clone)]
pub struct StopTime {
    pub trip_idx: usize,
    pub stop_idx: usize,
    pub stop_sequence: u32,
    pub arrival_secs: Option<u32>,
    pub departure_secs: Option<u32>,
    pub stop_headsign: String,
    pub pickup_type: Option<u16>,
    pub drop_off_type: Option<u16>,
}


#[derive(Debug, Clone)]
pub struct Shape {
    pub points: Vec<(f64, f64)>, // (lat, lon) in shape_pt_sequence order
}


#[derive(Debug, Clone)]
pub struct Connection{
    // kinda redundant, but simple
    pub dep_stop: usize,
    pub arr_stop: usize,
    pub dep_time: u32,
    pub arr_time: u32,
    pub trip_idx: usize,
}

#[derive(Debug, Clone)]
pub struct RaptorRoute {
    pub route_idx: usize,
    pub stops: Vec<usize>,
    pub trips: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Graph {
    pub stops: Vec<Stop>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    pub shapes: Vec<Shape>,
    pub services: Vec<String>,
    pub stations: Vec<Station>,

    pub source_dir: String,
    pub stops_by_id: HashMap<String, usize>,
    pub routes_by_id: HashMap<String, usize>,
    pub trips_by_id: HashMap<String, usize>,
    pub shapes_by_id: HashMap<String, usize>,
    pub services_by_id: HashMap<String, usize>,

    pub trips_by_route: Vec<Vec<usize>>,
    pub stop_times_by_trip: Vec<Vec<StopTime>>,
    pub stop_times_by_stop: Vec<Vec<(usize, usize)>>,

    pub stops_by_route: HashMap<(usize, usize), Vec<usize>>,
    pub times_at: Vec<HashMap<usize, (u32, u32)>>,

    pub connections: Vec<Connection>,
    pub raptor_routes: Vec<RaptorRoute>,
    pub rroutes_by_stop: Vec<Vec<(usize, usize)>>,
}

impl Graph {
    pub fn load(dir: &Path) -> Result<Self> {
        let raw_stops: Vec<RawStop>         = load_csv(&dir.join("stops.txt"))?;
        let raw_routes: Vec<RawRoute>       = load_csv(&dir.join("routes.txt"))?;
        let raw_trips: Vec<RawTrip>         = load_csv(&dir.join("trips.txt"))?;
        let raw_stop_times: Vec<RawStopTime> = load_csv(&dir.join("stop_times.txt"))?;
        let raw_shapes: Vec<RawShapePoint>  = load_csv(&dir.join("shapes.txt"))?;

        let mut graph = Self {
            source_dir: dir.to_string_lossy().into_owned(),
            ..Default::default()
        };

        graph.build(raw_stops, raw_routes, raw_trips, raw_stop_times, raw_shapes);
        Ok(graph)
    }

    fn build(
        &mut self,
        raw_stops: Vec<RawStop>,
        raw_routes: Vec<RawRoute>,
        raw_trips: Vec<RawTrip>,
        raw_stop_times: Vec<RawStopTime>,
        raw_shape_points: Vec<RawShapePoint>,
    ) {
        self.stops_by_id = raw_stops
            .iter()
            .enumerate()
            .map(|(i, s)| (s.stop_id.clone(), i))
            .collect();

        let mut stations_set: HashMap<String, Station> = HashMap::new();
        self.stops = raw_stops
            .into_iter()
            .enumerate()
            .map(|(idx, stop)| {
                let mut station_idx = 0;
                if let Some(cur_station) = stations_set.get_mut(&stop.stop_name){
                    cur_station.stops.push(idx);
                    station_idx = cur_station.idx;
                }
                else{
                    station_idx = stations_set.len();
                    stations_set.insert(stop.stop_name.clone(), Station{idx: station_idx, name: stop.stop_name.clone(), stops: vec![idx]});
                }

                Stop {
                    idx,
                    station: station_idx,
                    stop_code: stop.stop_code,
                    name: stop.stop_name,
                    stop_desc: stop.stop_desc,
                    stop_lat: stop.stop_lat,
                    stop_lon: stop.stop_lon,
                    zone_id: stop.zone_id,
                    stop_url: stop.stop_url,
                    foothpaths: Vec::new(),
                    location_type: stop.location_type,
                    stop_timezone: stop.stop_timezone,
                    wheelchair_boarding: stop.wheelchair_boarding,
                    platform_code: stop.platform_code,
                }
            })
            .collect();
        self.stations = stations_set.into_values().collect();
        self.stations.sort_by_key(|k| k.idx);

        for station in &self.stations{
            for i in 0..station.stops.len(){
                let i = station.stops[i];
                self.stops[i].foothpaths.push((i, 60)); // min changeover 1 minutes
                for j in (i+1)..station.stops.len(){
                    let j = station.stops[j];
                    self.stops[i].foothpaths.push((j,180)); // min stop change 3 minutes
                }
            }
        }

        self.routes_by_id = raw_routes
            .iter()
            .enumerate()
            .map(|(i, r)| (r.route_id.clone(), i))
            .collect();

        self.routes = raw_routes
            .into_iter()
            .enumerate()
            .map(|(i, r)| Route {
                idx: i,
                agency_id: r.agency_id,
                route_short_name: r.route_short_name,
                route_long_name: r.route_long_name,
                route_type: r.route_type,
            })
            .collect();

        self.trips_by_route = vec![Vec::new(); self.routes.len()];

        {
            let mut buckets: HashMap<String, Vec<RawShapePoint>> = HashMap::default();
            for p in raw_shape_points {
                buckets.entry(p.shape_id.clone()).or_default().push(p);
            }
            let mut shape_list: Vec<(String, Vec<RawShapePoint>)> =
                buckets.into_iter().collect();
            shape_list.sort_by(|a, b| a.0.cmp(&b.0));

            self.shapes = shape_list
                .into_iter()
                .enumerate()
                .map(|(i, (shape_id, mut pts))| {
                    pts.sort_by_key(|p| p.shape_pt_sequence);
                    let points = pts
                        .into_iter()
                        .map(|p| (p.shape_pt_lat, p.shape_pt_lon))
                        .collect();
                    self.shapes_by_id.insert(shape_id, i);
                    Shape { points }
                })
                .collect();
        }

        self.trips_by_id = raw_trips
            .iter()
            .enumerate()
            .map(|(i, t)| (t.trip_id.clone(), i))
            .collect();

        self.trips = raw_trips
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let route_idx = self
                    .routes_by_id
                    .get(&r.route_id)
                    .copied()
                    .unwrap_or(0);

                let service_idx = *self
                    .services_by_id
                    .entry(r.service_id.clone())
                    .or_insert_with(|| {
                        let idx = self.services.len();
                        self.services.push(r.service_id);
                        idx
                    });

                let shape_idx = if r.shape_id.is_empty() {
                    None
                } else {
                    self.shapes_by_id.get(&r.shape_id).copied()
                };

                if route_idx < self.trips_by_route.len() {
                    self.trips_by_route[route_idx].push(i);
                }

                Trip {
                    idx: i,
                    route_idx,
                    service_idx,
                    trip_headsign: r.trip_headsign,
                    direction_id: r.direction_id.unwrap_or(0),
                    shape_idx,
                }
            })
            .collect();

        self.stop_times_by_trip = vec![Vec::new(); self.trips.len()];
        self.stop_times_by_stop = vec![Vec::new(); self.stops.len()];
        self.times_at           = vec![HashMap::default(); self.trips.len()];

        for r in raw_stop_times {
            let trip_idx = match self.trips_by_id.get(&r.trip_id) {
                Some(&i) => i,
                None => {
                    eprintln!("Warning: stop_time references unknown trip {}", r.trip_id);
                    continue;
                }
            };
            let stop_idx = match self.stops_by_id.get(&r.stop_id) {
                Some(&i) => i,
                None => {
                    eprintln!("Warning: stop_time references unknown stop {}", r.stop_id);
                    continue;
                }
            };

            let arrival_secs   = parse_time(&r.arrival_time);
            let departure_secs = parse_time(&r.departure_time);

            if let (Some(arr), Some(dep)) = (arrival_secs, departure_secs) {
                self.times_at[trip_idx].insert(stop_idx, (arr, dep));
            }

            self.stop_times_by_trip[trip_idx].push(StopTime {
                trip_idx,
                stop_idx,
                stop_sequence: r.stop_sequence,
                arrival_secs,
                departure_secs,
                stop_headsign: r.stop_headsign,
                pickup_type:   r.pickup_type,
                drop_off_type: r.drop_off_type,
            });
        }

        for trip in &self.stop_times_by_trip {
            let mut prev_stop_time: Option<(usize, u32)> = None; //arrival, departure time
            for cur in trip{
                if let Some(prev_stop_time) = prev_stop_time {
                    let cur_arrival = cur.arrival_secs.unwrap();
                   self.connections.push(Connection { dep_stop: prev_stop_time.0, arr_stop: cur.stop_idx, dep_time: prev_stop_time.1, arr_time: cur_arrival, trip_idx: cur.trip_idx });
                }
                prev_stop_time = Some((cur.stop_idx, cur.arrival_secs.unwrap()));
            }
        }

        self.connections.sort_by_key(|k| k.dep_time);

        for trip_idx in 0..self.stop_times_by_trip.len() {
            self.stop_times_by_trip[trip_idx].sort_by_key(|st| st.stop_sequence);

            for (pos, st) in self.stop_times_by_trip[trip_idx].iter().enumerate() {
                self.stop_times_by_stop[st.stop_idx].push((trip_idx, pos));
            }
        }

        for trip in &self.trips {
            let stop_idxs: Vec<usize> = self.stop_times_by_trip[trip.idx]
                .iter()
                .map(|st| st.stop_idx)
                .collect();

            let key = (trip.route_idx, trip.direction_id as usize);
            self.stops_by_route
                .entry(key)
                .and_modify(|existing| {
                    if stop_idxs.len() > existing.len() {
                        *existing = stop_idxs.clone();
                    }
                })
                .or_insert(stop_idxs);
        }

        let mut routes_map: HashMap<(usize, Vec<usize>), Vec<usize>> = HashMap::new();
        for trip in &self.trips {
            let stops: Vec<usize> = self.stop_times_by_trip[trip.idx]
                .iter()
                .map(|st| st.stop_idx)
                .collect();
            if stops.is_empty() {
                continue;
            }
            routes_map.entry((trip.route_idx, stops)).or_default().push(trip.idx);
        }

        let mut raptor_routes = Vec::new();
        for ((route_idx, stops), mut trips) in routes_map {
            trips.sort_by_key(|&t_idx| self.departure_at(t_idx, stops[0]).unwrap_or(0));
            raptor_routes.push(RaptorRoute {
                route_idx,
                stops,
                trips,
            });
        }
        self.raptor_routes = raptor_routes;

        let mut rroutes_by_stop = vec![Vec::new(); self.stops.len()];
        for (r_idx, r) in self.raptor_routes.iter().enumerate() {
            for (pos, &stop_id) in r.stops.iter().enumerate() {
                rroutes_by_stop[stop_id].push((r_idx, pos));
            }
        }
        self.rroutes_by_stop = rroutes_by_stop;
    }

    pub fn arrival_at(&self, trip_idx: usize, stop_idx: usize) -> Option<u32> {
        self.times_at.get(trip_idx)?.get(&stop_idx).map(|&(arr, _)| arr)
    }

    pub fn departure_at(&self, trip_idx: usize, stop_idx: usize) -> Option<u32> {
        self.times_at.get(trip_idx)?.get(&stop_idx).map(|&(_, dep)| dep)
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

pub fn parse_time(s: &str) -> Option<u32> {
    let mut parts = s.splitn(3, ':');
    let h:   u32 = parts.next()?.parse().ok()?;
    let m:   u32 = parts.next()?.parse().ok()?;
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
            Ok(v)  => out.push(v),
            Err(e) => eprintln!("Warning: skipping row – {e}"),
        }
    }
    Ok(out)
}
