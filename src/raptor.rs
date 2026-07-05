use crate::footpaths::Footpaths;
use crate::graph::Graph;
use crate::journey::{Journey, Leg};
use crate::utils::Secs;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::vec;

pub struct Raptor<'a> {
    graph: &'a Graph,
}

#[derive(Debug, Clone)]
enum Parent {
    Trip {
        arrival_time: Secs,
        boarding_stop: usize,
        trip_idx: usize,
    },
    Walk {
        arrival_time: Secs,
        from_stop: usize,
        duration: Secs,
    },
}

impl<'a> Raptor<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph}
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
                            parent[p] = Some(Parent::Trip {
                                arrival_time: at,
                                trip_idx: t.unwrap().0,
                                boarding_stop: t.unwrap().1,
                            });
                            marked_stops.insert(p);
                        }
                    }

                    if tau[k-1][p] <= at {
                        t = self.et(*r, p, tau[k-1][p]).map(|t| (t, p));
                    }
                }
            }

            for from in &marked_stops.clone() {
                if let Some(footpaths) = self.graph.footpaths.get(from) {
                    for (to, time) in footpaths {
                        if to == from || *time == 30{
                            continue;
                        }
                        let arrival = tau[k][*from] + *time;
                        if arrival < tau[k][*to] && arrival < tau_best[*to] {
                            tau[k][*to] = arrival;
                            tau_best[*to] = arrival;
                            parent[*to] = Some(Parent::Walk {
                                arrival_time: arrival,
                                from_stop: *from,
                                duration: *time,
                            });
                            marked_stops.insert(*to);
                        }
                    }
                }
            }

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
                Some(Parent::Trip {
                    arrival_time,
                    boarding_stop,
                    trip_idx,
                }) => {
                    let route_idx = self.graph.trips[*trip_idx].route_idx;
                    let trip_headsign = &self.graph.trips[*trip_idx].trip_headsign;
                    let route_name = &self.graph.routes[route_idx].route_short_name;

                    legs.push(Leg::first(*arrival_time, current_stop, self.graph.stops[current_stop].name.clone(), *trip_idx, trip_headsign.clone(), route_name.clone()));

                    let dep_time = self.graph.departure_at(*trip_idx, *boarding_stop)?;
                    legs.push(Leg::first(dep_time, *boarding_stop, self.graph.stops[*boarding_stop].name.clone(), *trip_idx, trip_headsign.clone(), route_name.clone()));

                    current_stop = *boarding_stop;
                }
                Some(Parent::Walk {
                    arrival_time,
                    from_stop,
                    duration,
                }) => {
                    legs.push(Leg::second(*arrival_time, current_stop, self.graph.stops[current_stop].name.clone()));
                    legs.push(Leg::second(arrival_time - *duration, *from_stop, self.graph.stops[*from_stop].name.clone()));

                    current_stop = *from_stop;
                }
                None => break,
            }
        }
        legs.reverse();

        Some(Journey { legs, arrival })
    }
}
