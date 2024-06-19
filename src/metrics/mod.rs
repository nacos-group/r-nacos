pub mod core;
pub mod metrics_type;
pub mod counter;
mod gauge;
mod histogram;



#[derive(Default,Debug,Clone)]
pub struct CounterItem(pub(crate)u64);

impl CounterItem {
    pub fn increment(&mut self, value: u64) {
        self.0 += value;
    }

    pub fn absolute(&mut self, value: u64) {
        self.0 = value;
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl From<CounterItem> for u64 {
    fn from(value: CounterItem) -> Self {
        value.0
    }
}

impl From<u64> for CounterItem {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Default,Debug,Clone)]
pub struct GaugeItem(pub(crate)f64);

impl GaugeItem {
    /// Increments the gauge by the given amount.
    fn increment(&mut self, value: f64){
        self.0+=value;
    }

    /// Decrements the gauge by the given amount.
    fn decrement(&mut self, value: f64) {
        self.0-=value;
    }

    /// Sets the gauge to the given amount.
    fn set(&mut self, value: f64) {
        self.0 = value;
    }
}

impl From<GaugeItem> for f64 {
    fn from(value: GaugeItem) -> Self {
        value.0
    }
}

impl From<f64> for GaugeItem {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
