use crate::graph::{parse_time, Graph, StopTime, Trip, Stop};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
pub type Secs = u32;

#[derive(Debug, Clone)]
pub struct Leg{
    pub time: Secs,
    pub stop_name: String,
    pub route_id: String,
    pub route_headline: String,
    pub route_name: String,
}
impl Leg{
    fn new(time: Secs, stop_name: String, route_id: String) -> Self{
        //TODO: resolve rest of the data from route_id 
        Self{
            time,
            stop_name,
            route_id,
            route_headline: "Bronowice".to_string(),
            route_name: "1".to_string()
        }
    }
}
pub struct Journey {
    pub legs: Vec<Leg>,
    pub arrival: Secs,
}

pub struct Raptor<'a> {
    graph: &'a Graph,
    crossings: HashMap<(String, String), Secs>,
}

const MAX_ROUNDS: usize = 1;

fn genCrossings(graph: &Graph) -> HashMap<(String, String), Secs> {
    let mut crossings = HashMap::new();
    for c1 in &graph.stops {
        for c2 in &graph.stops {
            if c1.stop_name == c2.stop_name {
                crossings.insert((c1.stop_id.clone(), c2.stop_id.clone()), 0);
            }
        }
    }
    crossings
}

impl<'a> Raptor<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
            crossings: genCrossings(graph),
        }
    }

    pub fn et(&self, route_id: &str, dir: usize, stop_id: &str, tau: Secs) -> Option<&Trip> {
        let trip_indices = self.graph.trips_by_route.get(route_id)?;

        let mut best_trip: Option<&Trip> = None;
        let mut best_dep = Secs::MAX;

        for &ti in trip_indices {
            let trip = &self.graph.trips[ti];
            if trip.direction_id.unwrap() != dir as u16 {
                continue;
            }

            let Some(stop_times) = self.graph.stop_times_by_trip.get(&trip.trip_id) else {
                continue;
            };

            let Some(st) = stop_times.iter().find(|st| st.stop_id == stop_id) else {
                continue;
            };

            let Some(dep) = parse_time(&st.departure_time) else { continue };

            if dep >= tau && dep < best_dep {
                best_dep = dep;
                best_trip = Some(trip);
            }
        }

        best_trip
    }

    fn scan_route(
        &self,
        route: &(String, usize),
        tau_prev: &HashMap<String, Secs>,
        tau_k: &mut HashMap<String, Secs>,
        parent_k: &mut HashMap<String, Option<(Secs, String)>>,
    ) {
        let route_id = &route.0;
        let dir = route.1;
        let mut current_trip: Option<(&Trip, &Stop)> = None;

        let route_stops = self.graph.stops_by_route.get(&route).unwrap();
        for &stop_idx in route_stops {
            let stop_id = &self.graph.stops[stop_idx].stop_id;

            if current_trip.is_none() {
                // find first trip that arrives at this stop
                let trip = self.et(route_id, dir, stop_id, tau_prev[stop_id]);
                current_trip = trip.map(|trip| (trip, &self.graph.stops[stop_idx]));
            }

            if let Some(trip) = current_trip {
                let arrival_time = match self.graph.arrival_at(&trip.0.trip_id, stop_id) {
                    Some(time) => time,
                    None => return,
                };
                // check if we can switch to a faster trip
                if tau_prev[stop_id] < arrival_time {
                    let first_trip = self.et(route_id, dir, stop_id, tau_prev[stop_id]);
                    current_trip = first_trip.map(|trip| (trip, &self.graph.stops[stop_idx]));
                }
            }

            if let Some(trip) = current_trip {
                let arrival_time = match self.graph.arrival_at(&trip.0.trip_id, stop_id) {
                    Some(time) => time,
                    None => return,
                };
                // update the time for the stop
                if let Some(val) = tau_k.get_mut(stop_id) {
                    if arrival_time < *val {
                        *val = arrival_time;
                        *parent_k.get_mut(stop_id).unwrap() = Some((arrival_time, trip.1.stop_id.clone()));
                    }
                }
            }
        }
    }

    fn update_crossings(&mut self, tau: &mut HashMap<String, Secs>) {
        for ((c1, c2), &l) in &self.crossings {
            let t1 = tau[c1];
            let t2 = tau[c2];
            *tau.get_mut(c1).unwrap() = min(t1, t2 + l);
        }
    }

    pub fn query(&mut self, from_stop: &str, to_stop: &str, departure: Secs) -> Option<Journey> {
        println!("from_stop: {}, to_stop: {}, departure: {}", from_stop, to_stop, departure);

        let mut tau: HashMap<String, Secs> = self.graph.stops.iter()
            .map(|s| (s.stop_id.clone(), Secs::MAX))
            .collect();
        *tau.get_mut(from_stop).unwrap() = departure;

        let mut parent: HashMap<String, Option<(Secs, String)>> = self.graph.stops.iter()
            .map(|s| (s.stop_id.clone(), None))
            .collect();

        for _k in 0..2 {
            let tau_prev = tau.clone();
            for route in self.graph.stops_by_route.keys() {
                self.scan_route(route, &tau_prev, &mut tau, &mut parent);
            }
            self.update_crossings(&mut tau);
        }

        let arrival = tau[to_stop];
        if arrival == Secs::MAX {
            return None;
        }

        let stop_name = |id: &str| {
            return if let Some(idx) = self.graph.stops_by_id.get(id) {
                self.graph.stops[*idx].stop_name.clone()
            } else {
                println!("missing id: {}", id);
                "".to_string()
            }
        };

        let mut legs: Vec<Leg> = Vec::new();
        let mut current = (arrival, to_stop.to_string());

        //TODO: fill the route id
        while current.1 != from_stop {
            legs.push(Leg::new(current.0, stop_name(current.1.as_str()), "route_id".to_string()));
            match parent[current.1.as_str()].clone() {
                Some(p) => current = p,
                None => break,
            }
            legs.push(Leg::new(current.0, stop_name(current.1.as_str()), "route_id".to_string()));
        }
        legs.push(Leg::new(departure, stop_name(from_stop), "route_id".to_string()));
        legs.reverse();

        Some(Journey { legs, arrival })
    }

}
