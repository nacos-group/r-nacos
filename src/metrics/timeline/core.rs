use crate::metrics::timeline::model::{MetricsSnapshot, TimelineGroup};

#[derive(Debug, Clone, Default)]
pub struct MetricsTimelineManager {
    minute_timeline_group: TimelineGroup,
    least_timeline_group: TimelineGroup,
}

impl MetricsTimelineManager {
    pub fn new() -> Self {
        Self {
            minute_timeline_group: TimelineGroup::new(360),
            least_timeline_group: TimelineGroup::new(360),
        }
    }

    pub fn add_minute_record(&mut self, snapshot: MetricsSnapshot) {
        self.minute_timeline_group.add_record(snapshot);
    }

    pub fn add_least_record(&mut self, snapshot: MetricsSnapshot) {
        self.minute_timeline_group.add_record(snapshot);
    }

    pub fn last_minute_record_time(&self) -> u64 {
        self.minute_timeline_group.last_time
    }
}
