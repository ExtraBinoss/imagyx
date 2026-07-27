use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, Once, OnceLock},
    time::Instant,
};

use super::helpers::{MAX_SAMPLES_PER_SPAN, percentile};

static INIT: Once = Once::new();
static METRICS: OnceLock<Mutex<HashMap<&'static str, SpanSamples>>> = OnceLock::new();
const SUMMARY_INTERVAL: usize = 32;

#[derive(Debug, Default)]
struct SpanSamples {
    total: usize,
    values: VecDeque<u64>,
}

#[derive(Debug)]
pub struct TraceSpan {
    name: &'static str,
    started_at: Instant,
}

impl Drop for TraceSpan {
    fn drop(&mut self) {
        let elapsed_ms = self.started_at.elapsed().as_millis() as u64;
        let summary = record(self.name, elapsed_ms);
        eprintln!("[imagyx:trace] span={} elapsed_ms={elapsed_ms}", self.name);
        if let Some((count, p50, p95)) = summary {
            eprintln!(
                "[imagyx:trace] summary={} samples={count} p50_ms={p50} p95_ms={p95}",
                self.name
            );
        }
    }
}

pub fn init() {
    INIT.call_once(|| eprintln!("[imagyx:trace] development tracing enabled"));
}

pub fn span(name: &'static str) -> TraceSpan {
    TraceSpan {
        name,
        started_at: Instant::now(),
    }
}

pub fn event(name: &'static str, detail: impl std::fmt::Display) {
    eprintln!("[imagyx:trace] event={name} detail={detail}");
}

#[cfg(test)]
fn snapshot(name: &'static str) -> Option<(usize, u64, u64)> {
    let metrics = METRICS.get()?.lock().ok()?;
    let samples = &metrics.get(name)?.values;
    Some((
        samples.len(),
        percentile(samples, 0.50)?,
        percentile(samples, 0.95)?,
    ))
}

fn record(name: &'static str, elapsed_ms: u64) -> Option<(usize, u64, u64)> {
    let metrics = METRICS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut metrics) = metrics.lock() else {
        return None;
    };
    let samples = metrics.entry(name).or_default();
    samples.total = samples.total.saturating_add(1);
    if samples.values.len() == MAX_SAMPLES_PER_SPAN {
        samples.values.pop_front();
    }
    samples.values.push_back(elapsed_ms);

    if samples.total % SUMMARY_INTERVAL != 0 {
        return None;
    }
    Some((
        samples.values.len(),
        percentile(&samples.values, 0.50)?,
        percentile(&samples.values, 0.95)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{record, snapshot};

    #[test]
    fn records_bounded_p50_and_p95_samples() {
        for value in 1..=300 {
            record("test.span", value);
        }
        let (count, p50, p95) = snapshot("test.span").expect("snapshot");
        assert_eq!(count, 256);
        assert!(p50 >= 170);
        assert!(p95 >= p50);
    }
}
