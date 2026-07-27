pub(super) const MAX_SAMPLES_PER_SPAN: usize = 256;

pub(super) fn percentile(
    samples: &std::collections::VecDeque<u64>,
    percentile: f32,
) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort_unstable();
    let position = ((sorted.len() - 1) as f32 * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted.get(position).copied()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::percentile;

    #[test]
    fn computes_nearest_rank_percentiles() {
        let samples = VecDeque::from([1, 2, 3, 4, 100]);
        assert_eq!(percentile(&samples, 0.50), Some(3));
        assert_eq!(percentile(&samples, 0.95), Some(100));
    }
}
