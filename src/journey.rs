use crate::utils::Secs;

#[derive(Debug, Clone)]
pub struct Leg{
    pub time: Secs,
    pub stop_idx: usize,
    pub stop_name: String,
    pub trip_idx: Option<usize>,
    pub trip_headline: Option<String>,
    pub route_name: Option<String>
}
impl Leg{
    pub fn first(time: Secs, stop_idx: usize, stop_name: String, trip_idx: usize, trip_headline: String, route_name: String) -> Self{
        Self{
            time,
            stop_idx,
            stop_name,
            trip_idx: Some(trip_idx),
            trip_headline: Some(trip_headline),
            route_name: Some(route_name),
        }
    }

    pub fn second(time: Secs, stop_idx: usize, stop_name: String) -> Self{
        Self{
            time,
            stop_idx,
            stop_name,
            trip_idx: None,
            trip_headline: None,
            route_name: None
        }
    }
}
pub struct Journey {
    pub legs: Vec<Leg>,
    pub arrival: Secs,
}


