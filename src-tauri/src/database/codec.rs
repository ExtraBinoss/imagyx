pub(super) fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

pub(super) fn decode_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{decode_vector, encode_vector};

    #[test]
    fn vector_blob_round_trip_is_lossless() {
        let vector = vec![0.25, -1.5, 8.0, f32::EPSILON];
        assert_eq!(decode_vector(&encode_vector(&vector)), vector);
    }

    #[test]
    fn ignores_incomplete_trailing_bytes() {
        let mut encoded = encode_vector(&[1.0, 2.0]);
        encoded.extend_from_slice(&[1, 2, 3]);
        assert_eq!(decode_vector(&encoded), vec![1.0, 2.0]);
    }
}
