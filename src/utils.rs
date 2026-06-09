use crate::raptor::Secs;

pub fn fmt_time(secs: Secs) -> String {
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}
