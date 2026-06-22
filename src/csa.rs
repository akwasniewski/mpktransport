use std::cmp::min;

use crate::{graph::{Connection, Graph}, journey::{Journey, Leg}, utils::Secs};
pub struct Csa<'a> {
    graph: &'a Graph,
}

#[derive(Debug, Default, Clone)]
struct JourneyMarker{
    c_enter: usize,
    c_exit: usize,
}

impl<'a> Csa<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
        }
    }

    fn get_journey(&self, target_stop: usize, arrival: Option<Secs>, j: &[Option<JourneyMarker>]) -> Option<Journey>{
        let legs: Vec<Leg> = Vec::new();
        let arrival = arrival?;
        let mut cur_stop = target_stop;
        while let Some(cur_j) = &j[cur_stop]{
            println!("{:?}", cur_j);
            cur_stop = self.graph.connections[cur_j.c_enter].dep_stop;
        }
        Some(Journey{
            legs,
            arrival
        })
    }

    pub fn query(&self, source_stop: usize, target_stop: usize, source_time: Secs) -> Option<Journey> {
        let mut s: Vec<Option<u32>> = vec![None; self.graph.stops.len()];
        s[source_stop] = Some(source_time);

        let mut t: Vec<Option<usize>> = vec![None; self.graph.trips.len()];

        let mut j: Vec<Option<JourneyMarker>> = vec![None; self.graph.stops.len()];
        let c0 = self.graph.connections.partition_point(|x| x.dep_time<source_time);
        for c_idx in c0..self.graph.connections.len(){
            let c = &self.graph.connections[c_idx];
            if s[target_stop].is_some_and(|target_arrival| target_arrival <= c.dep_time) {
               break; 
            }

            if t[c.trip_idx].is_some() || s[c.dep_stop].is_some_and(|dep_arrival| dep_arrival <= c.dep_time){
                if t[c.trip_idx].is_none(){
                    t[c.trip_idx] = Some(c_idx);
                }

                if s[c.arr_stop].is_none_or(|arr_arrival| c.arr_time < arr_arrival){
                    s[c.arr_stop] = Some(c.arr_time);
                    j[c.arr_stop] = Some(JourneyMarker{c_enter: t[c.trip_idx].unwrap(), c_exit: c_idx});
                }
            }

        }

        self.get_journey(target_stop, s[target_stop], &j)
    }
}
