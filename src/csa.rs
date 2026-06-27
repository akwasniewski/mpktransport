use std::cmp::{Reverse, min};

use crate::{graph::{Connection, Graph}, journey::{Journey, Leg}, utils::Secs};
pub struct Csa<'a> {
    graph: &'a Graph,
}

#[derive(Debug, Default, Clone)]
struct JourneyMarker{
    c_enter: usize,
    c_exit: usize,
    f_dur: Secs,
}

impl<'a> Csa<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
        }
    }

    fn get_journey(&self, arrival: Option<(usize, Secs)>, j: &[Option<JourneyMarker>]) -> Option<Journey>{
        let mut legs: Vec<Leg> = Vec::new();
        let arrival_time = arrival?.1;
        let mut cur_stop = arrival?.0;

        while let Some(cur_j) = &j[cur_stop]{
            let cur_c_enter = &self.graph.connections[cur_j.c_enter];
            let cur_c_exit = &self.graph.connections[cur_j.c_exit];

            let route_idx = self.graph.trips[cur_c_exit.trip_idx].route_idx;
    legs.push(Leg::first(cur_c_exit.arr_time, cur_c_exit.arr_stop, self.graph.stops[cur_c_exit.arr_stop].name.clone(), cur_c_exit.trip_idx, self.graph.trips[cur_c_exit.trip_idx].trip_headsign.clone(), self.graph.routes[route_idx].route_short_name.clone()));


            let route_idx = self.graph.trips[cur_c_enter.trip_idx].route_idx;
            legs.push(Leg::first(cur_c_enter.dep_time, cur_c_enter.dep_stop, self.graph.stops[cur_c_enter.dep_stop].name.clone(), cur_c_enter.trip_idx, self.graph.trips[cur_c_enter.trip_idx].trip_headsign.clone(),  self.graph.routes[route_idx].route_short_name.clone()));
   
            cur_stop = self.graph.connections[cur_j.c_enter].dep_stop;
        }
        legs.reverse();
        Some(Journey{
            legs,
            arrival: arrival_time
        })
    }

    pub fn query(&self, source_station: usize, target_station: usize, source_time: Secs) -> Option<Journey> {
        let mut s: Vec<Option<u32>> = vec![None; self.graph.stops.len()];
        for stop in &self.graph.stations[source_station].stops{
            s[*stop] = Some(source_time);
        }

        let mut t: Vec<Option<usize>> = vec![None; self.graph.trips.len()];

        let mut j: Vec<Option<JourneyMarker>> = vec![None; self.graph.stops.len()];
        let c0 = self.graph.connections.partition_point(|x| x.dep_time<source_time);
        for c_idx in c0..self.graph.connections.len(){
            let c = &self.graph.connections[c_idx];
            
            // that is not the ultimate solution, but it works for now
            for stop in &self.graph.stations[target_station].stops{
                if s[*stop].is_some_and(|target_arrival| target_arrival <= c.dep_time) {
                    break; 
                }
            }
       

            if t[c.trip_idx].is_some() || s[c.dep_stop].is_some_and(|dep_arrival| dep_arrival <= c.dep_time){
                if t[c.trip_idx].is_none(){
                    t[c.trip_idx] = Some(c_idx);
                }
                for f in &self.graph.stops[c.arr_stop].foothpaths{
                    if s[f.0].is_none_or(|arr_arrival| c.arr_time + f.1 < arr_arrival){
                        s[f.0] = Some(c.arr_time + f.1);
                        j[f.0] = Some(JourneyMarker{c_enter: t[c.trip_idx].unwrap(), c_exit: c_idx, f_dur: f.1});
                    }

                }
            }

        }
        let mut best_target: Option<(usize, Secs)> = None;
        for stop in &self.graph.stations[target_station].stops{
            if s[*stop].is_none(){ continue;}
            if best_target.is_none() || best_target?.1 > s[*stop]?{
                best_target = Some((*stop, s[*stop]?));
            }
        }
        println!("{:?}", best_target);
        self.get_journey(best_target, &j)
    }
}
