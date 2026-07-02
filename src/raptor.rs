
use crate::graph::{Graph};
use crate::journey::{Journey, Leg};
use crate::utils::Secs;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::vec;

pub struct Raptor<'a> {
    graph: &'a Graph,
    footpaths: HashMap<(usize, usize), Secs>,
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
            footpaths: gen_crossings(graph),
        }
    }

    fn et(&self, route: usize, stop_id: usize, tau: Secs) -> Option<usize> {
        self.graph.raptor_routes[route].trips
            .iter()
            .map(|&ti| &self.graph.trips[ti])
            .filter_map(|trip| {
                let dep = self.graph.departure_at(trip.idx, stop_id)?;
                if dep >= tau { Some((trip, dep)) } else { None }
            })
            .min_by_key(|&(_, dep)| dep)
            .map(|(trip, _)| trip.idx)
    }

    pub fn query(&mut self, source_station: usize, target_station: usize, departure: Secs) -> Option<Journey> {
        let max_transfers = 5;
        println!("source_station: {}, target_station: {}, departure: {}", source_station, target_station, departure);

        let from_stops = &self.graph.stations[source_station].stops;
        let to_stops = &self.graph.stations[target_station].stops;

        let mut tau: Vec<Vec<Secs>> = vec![vec![Secs::MAX; self.graph.stops.len()]; max_transfers + 1];
        let mut tau_best: Vec<Secs> = vec![Secs::MAX; self.graph.stops.len()];
        let mut parent: Vec<Option<Parent>> = vec![None; self.graph.stops.len()];

        let mut Q: HashMap<usize, usize> = HashMap::new();
        let mut marked_stops: HashSet<usize> = HashSet::new();

        for stop in from_stops {
            tau[0][*stop] = departure;
            marked_stops.insert(*stop);
        }

        for k in 1..max_transfers+1 {
            Q.clear();
            for p in &marked_stops {
                for (r, p_idx) in &self.graph.rroutes_by_stop[*p] {
                    let p1_idx = Q.get(r).unwrap_or(&usize::MAX);
                    if *p_idx < *p1_idx {
                        Q.insert(*r, *p_idx);
                    }
                }
            }
            marked_stops.clear();

            for (r, p_idx) in &Q {
                let mut t: Option<(usize, usize)> = None;

                let route_stops = &self.graph.raptor_routes[*r].stops;
                for pi in *p_idx..route_stops.len() {
                    let p = route_stops[pi];
                    let at = if t != None { self.graph.arrival_at(t.unwrap().0, p).unwrap() } else { Secs::MAX };

                    if t != None {
                        if at < min(tau_best[p], *to_stops.iter().filter_map(|&s| tau_best.get(s)).min().unwrap_or(&Secs::MAX)) {
                            tau[k][p] = at;
                            tau_best[p] = at;
                            parent[p] = Some(Parent { arrival_time: at, trip_idx: t.unwrap().0, stop_idx: t.unwrap().1 });
                            marked_stops.insert(p);
                        }
                    }

                    if tau[k-1][p] <= at {
                        t = self.et(*r, p, tau[k-1][p]).map(|t| (t, p));
                    }
                }
            }

          // TODO footpaths

          if marked_stops.is_empty() {
              break;
          }
        }

        let &target_stop = to_stops.iter().min_by_key(|&&s| tau_best[s])?;
        let arrival = tau_best[target_stop];
        if arrival == Secs::MAX {
            return None;
        }

        let mut legs: Vec<Leg> = Vec::new();
        let mut current_stop = target_stop;

        while !from_stops.iter().any(|&s| s == current_stop) {
            match &parent[current_stop] {
                Some(p) => {
                    let route_idx = self.graph.trips[p.trip_idx].route_idx;

                    // Reconstruct the arrival (deboarding) leg
                    legs.push(Leg::first(
                        p.arrival_time,
                        current_stop,
                        self.graph.stops[current_stop].name.clone(),
                        p.trip_idx,
                        self.graph.trips[p.trip_idx].trip_headsign.clone(),
                        self.graph.routes[route_idx].route_short_name.clone(),
                    ));

                    // Reconstruct the departure (boarding) leg
                    let dep_time = self.graph.departure_at(p.trip_idx, p.stop_idx)?;
                    legs.push(Leg::first(
                        dep_time,
                        p.stop_idx,
                        self.graph.stops[p.stop_idx].name.clone(),
                        p.trip_idx,
                        self.graph.trips[p.trip_idx].trip_headsign.clone(),
                        self.graph.routes[route_idx].route_short_name.clone(),
                    ));

                    current_stop = p.stop_idx;
                }
                None => break,
            }
        }
        legs.reverse();

        Some(Journey { legs, arrival })
    }
}
