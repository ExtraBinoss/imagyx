#[derive(Debug, Default)]
pub struct TraceSpan;

pub fn init() {}

pub fn span(_name: &'static str) -> TraceSpan {
    TraceSpan
}

pub fn event(_name: &'static str, _detail: impl std::fmt::Display) {}

pub fn snapshot(_name: &'static str) -> Option<(usize, u64, u64)> {
    None
}
