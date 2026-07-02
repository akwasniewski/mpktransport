use crate::graph::{Graph, Trip, RaptorRoute};
use crate::journey::{Journey, Leg};
use crate::utils::Secs;
use std::collections::{HashMap};

pub struct Raptor<'a> {
    graph: &'a Graph,
    crossings: HashMap<(usize, usize), Secs>,
}

#[derive(Debug, Clone)]
struct Parent{
    arrival_time: Secs,
    stop_idx: usize,
    trip_idx: usize
}

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

    fn et(&self, route: &RaptorRoute, stop_id: usize, tau: Secs) -> Option<&Trip> {
        let mut best_trip: Option<&Trip> = None;
        let mut best_dep = Secs::MAX;

        for &ti in &route.trips {
            let trip = &self.graph.trips[ti];
            let Some(dep) = self.graph.departure_at(trip.idx, stop_id) else {
                continue;
            };

            if dep >= tau && dep < best_dep {
                best_dep = dep;
                best_trip = Some(trip);
            }
        }

        best_trip
    }

    fn scan_route(
        &self,
        route: &RaptorRoute,
        tau_prev: &[Secs],
        tau_k: &mut[Secs],
        parent_k: &mut[Option<Parent>],
    ) {
        let mut current_trip: Option<(&Trip, usize)> = None;

        for &stop_id in &route.stops {
            if current_trip.is_none() {
                // find first trip that arrives at this stop
                let trip = self.et(route, stop_id, tau_prev[stop_id]);
                current_trip = trip.map(|trip| (trip, stop_id));
            }

            if let Some((trip, _boarding_stop_id)) = current_trip {
                let arrival_time = match self.graph.arrival_at(trip.idx, stop_id) {
                    Some(time) => time,
                    None => continue,
                };
                // check if we can switch to a faster trip
                if tau_prev[stop_id] < arrival_time {
                    let first_trip = self.et(route, stop_id, tau_prev[stop_id]);
                    current_trip = first_trip.map(|trip| (trip, stop_id));
                }
            }

            if let Some((trip, boarding_stop_id)) = current_trip {
                let arrival_time = match self.graph.arrival_at(trip.idx, stop_id) {
                    Some(time) => time,
                    None => continue,
                };
                // update the time for the stop
                if let Some(val) = tau_k.get_mut(stop_id) && arrival_time < *val  {
                    *val = arrival_time;
                    parent_k[stop_id] = Some(Parent{arrival_time, stop_idx: boarding_stop_id, trip_idx: trip.idx});
                }
            }
        }
    }

    fn update_crossings(&mut self, tau: &mut[Secs]) {
        todo!()
    }

    pub fn query(&mut self, source_station: usize, target_station: usize, departure: Secs) -> Option<Journey> {
        println!("source_station: {}, target_station: {}, departure: {}", source_station, target_station, departure);

        let from_stops = &self.graph.stations[source_station].stops;
        let to_stops = &self.graph.stations[target_station].stops;

        let mut tau: Vec<Secs> = vec![Secs::MAX; self.graph.stops.len()];
        for stop in from_stops {
            tau[*stop] = departure;
        }

        let mut parent: Vec<Option<Parent>> = vec![None; self.graph.stops.len()];

        let max_transfers = 5;
        for _k in 0..max_transfers {
            let tau_prev = tau.clone();
            for route in &self.graph.raptor_routes {
                self.scan_route(route, &tau_prev, &mut tau, &mut parent);
            }
            // self.update_crossings(&mut tau);
        }

        let &target_stop = to_stops.iter().min_by_key(|&&s| tau[s])?;
        let arrival = tau[target_stop];
        if arrival == Secs::MAX {
            return None;
        }

        let mut legs: Vec<Leg> = Vec::new();
        let mut current_stop = target_stop;
        while !from_stops.iter().any(|&s| s == current_stop) {
            let Some(p) = &parent[current_stop] else { break };

            let route_idx = self.graph.trips[p.trip_idx].route_idx;
            let route_name = &self.graph.routes[route_idx].route_short_name;
            let trip_headline = &self.graph.trips[p.trip_idx].trip_headsign;
            legs.push(Leg::first(p.arrival_time, current_stop, self.graph.stops[current_stop].name.clone(), p.trip_idx, trip_headline.clone(), route_name.clone()));
            let dep_time = self.graph.departure_at(p.trip_idx, p.stop_idx)?;
            legs.push(Leg::first(dep_time, p.stop_idx, self.graph.stops[p.stop_idx].name.clone(), p.trip_idx, trip_headline.clone(), route_name.clone()));

            current_stop = p.stop_idx;
        }
        legs.reverse();

        Some(Journey { legs, arrival })
    }
}
