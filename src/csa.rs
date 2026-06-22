use std::cmp::min;

use crate::{graph::Graph, journey::Journey, utils::Secs};
pub struct Csa<'a> {
    graph: &'a Graph,
}

impl<'a> Csa<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
        }
    }

    pub fn query(&self, source_stop: usize, target_stop: usize, source_time: Secs) -> Option<Journey> {
        let mut s: Vec<Option<u32>> = vec![None; self.graph.stops.len()];
        s[source_stop] = Some(source_time);

        let mut t: Vec<bool> = vec![false; self.graph.trips.len()];
        let c0 = self.graph.connections.partition_point(|x| x.dep_time<source_time);
        for c in c0..self.graph.connections.len(){
            let c = &self.graph.connections[c];
            if s[target_stop].is_some_and(|target_arrival| target_arrival <= c.dep_time) {
                println!("arr time: {:?}",s[target_stop]);
                return None;
            }

            if t[c.trip_idx] || s[c.dep_stop].is_some_and(|dep_arrival| dep_arrival <= c.dep_time){
                t[c.trip_idx] = true;
                s[c.arr_stop] = Some(min(s[c.arr_stop].unwrap_or(Secs::MAX), c.arr_time));
            }

        }
        None
    }
}
