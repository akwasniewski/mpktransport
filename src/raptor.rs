use crate::graph::{Graph, Trip, Stop};
use crate::journey::{Journey, Leg};
use crate::utils::Secs;
use std::cmp::min;
use std::collections::{HashMap};

pub struct Raptor<'a> {
    graph: &'a Graph,
    crossings: HashMap<(usize, usize), Secs>,
}

#[derive(Debug, Clone)]
struct Parent{
    arrival_time: Secs,
    idx: usize,
    trip_idx: usize
}

const MAX_ROUNDS: usize = 1;

fn gen_crossings(graph: &Graph) -> HashMap<(usize, usize), Secs> {
    let mut crossings = HashMap::new();
    for c1 in &graph.stops {
        for c2 in &graph.stops {
            if c1.station == c2.station {
                crossings.insert((c1.idx, c2.idx), 0);
            }
        }
    }
    crossings
}

impl<'a> Raptor<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
            crossings: gen_crossings(graph),
        }
    }

    pub fn et(&self, route_id: usize, dir: usize, stop_id: usize, tau: Secs) -> Option<&Trip> {
        let trip_indices = self.graph.trips_by_route.get(route_id)?;

        let mut best_trip: Option<&Trip> = None;
        let mut best_dep = Secs::MAX;

        for &ti in trip_indices {
            let trip = &self.graph.trips[ti];
            if trip.direction_id != dir as u16 {
                continue;
            }

            let Some(stop_times) = self.graph.stop_times_by_trip.get(trip.idx) else {
                continue;
            };

            let Some(st) = stop_times.iter().find(|st| st.stop_idx == stop_id) else {
                continue;
            };

            let Some(dep) = st.departure_secs else { continue };

            if dep >= tau && dep < best_dep {
                best_dep = dep;
                best_trip = Some(trip);
            }
        }

        best_trip
    }

    fn scan_route(
        &self,
        route: &(usize, usize),
        tau_prev: &[Secs],
        tau_k: &mut[Secs],
        parent_k: &mut[Option<Parent>],
    ) {
        let route_id = route.0;
        let dir = route.1;
        let mut current_trip: Option<(&Trip, &Stop)> = None;

        let route_stops = self.graph.stops_by_route.get(route).unwrap();
        for &stop_idx in route_stops {
            let stop_id = self.graph.stops[stop_idx].idx;

            if current_trip.is_none() {
                // find first trip that arrives at this stop
                let trip = self.et(route_id, dir, stop_id, tau_prev[stop_id]);
                current_trip = trip.map(|trip| (trip, &self.graph.stops[stop_idx]));
            }

            if let Some(trip) = current_trip {
                let arrival_time = match self.graph.arrival_at(trip.0.idx, stop_id) {
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
                let arrival_time = match self.graph.arrival_at(trip.0.idx, stop_id) {
                    Some(time) => time,
                    None => return,
                };
                // update the time for the stop
                if let Some(val) = tau_k.get_mut(stop_id) && arrival_time < *val  {
                    *val = arrival_time;
                    parent_k[stop_idx] = Some(Parent{arrival_time, idx: trip.1.idx, trip_idx: trip.0.idx});
                }
            }
        }
    }

    fn update_crossings(&mut self, tau: &mut[Secs]) {
        for ((c1, c2), &l) in &self.crossings {
            let t1 = tau[*c1];
            let t2 = tau[*c2];
            tau[*c1] = min(t1, t2 + l);
        }
    }

    //TODO: fix to work with stations
    pub fn query(&mut self, from_stop: usize, to_stop: usize, departure: Secs) -> Option<Journey> {
        println!("from_stop: {}, to_stop: {}, departure: {}", from_stop, to_stop, departure);

        let mut tau: Vec<Secs> = vec![Secs::MAX; self.graph.stops.len()];
        tau[from_stop] = departure;

        let mut parent: Vec<Option<Parent>> = vec![None; self.graph.stops.len()];

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

        let mut legs: Vec<Leg> = Vec::new();
        let mut current = Parent{arrival_time:arrival, idx:to_stop, trip_idx: 0};

        while current.idx != from_stop {
            let route_idx = self.graph.trips[current.trip_idx].route_idx;
            legs.push(Leg::first(current.arrival_time, current.idx, self.graph.stops[current.idx].name.clone(), current.trip_idx, self.graph.trips[current.trip_idx].trip_headsign.clone(), self.graph.routes[route_idx].route_short_name.clone()));
            match parent[current.idx].clone() {
                Some(p) => current = p,
                None => break,
            }
            let route_idx = self.graph.trips[current.trip_idx].route_idx;
            legs.push(Leg::first(current.arrival_time, current.idx, self.graph.stops[current.idx].name.clone(), current.trip_idx, self.graph.trips[current.trip_idx].trip_headsign.clone(), self.graph.routes[route_idx].route_short_name.clone()));
        }

        legs.push(Leg::second(departure, from_stop,self.graph.stops[from_stop].name.clone()));
        legs.reverse();

        Some(Journey { legs, arrival })
    }

}
