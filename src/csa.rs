use crate::{graph::{Graph}, journey::{Journey, Leg}, utils::Secs};
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
            if cur_stop != cur_c_exit.arr_stop{ 
                legs.push(Leg::second(cur_c_exit.arr_time, cur_stop, self.graph.stops[cur_stop].name.clone()));
                legs.push(Leg::second(cur_c_exit.arr_time - cur_j.f_dur, cur_c_exit.arr_stop, self.graph.stops[cur_c_exit.arr_stop].name.clone()));
            }
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

    // pub fn query_multiple_places(&self, source_places: &Vec<usize>, target_places: &Vec<usize>, source_time: Secs) -> Option<Journey> {
    //     for 
    // }

    pub fn query(&self, source_station: usize, target_station: usize, source_time: Secs) -> Option<Journey> {
        let mut s: Vec<Option<u32>> = vec![None; self.graph.stops.len()];
        for stop in &self.graph.stations[source_station].stops{
            s[*stop] = Some(source_time);
        }

        let mut t: Vec<Option<usize>> = vec![None; self.graph.trips.len()];

        let mut j: Vec<Option<JourneyMarker>> = vec![None; self.graph.stops.len()];
        let c0 = self.graph.connections.partition_point(|x| x.dep_time<source_time);
        let mut best_target_arrival: Option<(usize, Secs)> = None;

        for c_idx in c0..self.graph.connections.len(){
            let c = &self.graph.connections[c_idx];

            if best_target_arrival.is_some_and(|best_arr| best_arr.1 <= c.dep_time){
                break;
            }       

            if t[c.trip_idx].is_some() || s[c.dep_stop].is_some_and(|dep_arrival| dep_arrival <= c.dep_time){
                if t[c.trip_idx].is_none(){
                    t[c.trip_idx] = Some(c_idx);
                }
                if let Some(footpaths) = self.graph.footpaths.get(&c.arr_stop) {
                    for (to, time) in footpaths {
                        if s[*to].is_none_or(|arr_arrival| c.arr_time + time < arr_arrival){
                            s[*to] = Some(c.arr_time + time);
                            j[*to] = Some(JourneyMarker{c_enter: t[c.trip_idx].unwrap(), c_exit: c_idx, f_dur: *time});
                        }

                        if self.graph.stops[*to].station == target_station 
                            && best_target_arrival.is_none_or(|best_arr| best_arr.1 > c.arr_time + time){
                            best_target_arrival = Some((*to, c.arr_time + time));
                        }
                        
                    }
                }
            }

        }

        self.get_journey(best_target_arrival, &j)
    }
}
