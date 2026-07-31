use super::VectorStore;
use crate::models::VectorEntry;

fn entry(image_id: &str, folder_id: &str, vector: Vec<f32>) -> VectorEntry {
    VectorEntry {
        image_id: image_id.into(),
        folder_id: folder_id.into(),
        vector,
    }
}

#[test]
fn keeps_vectors_contiguous_and_returns_top_k() {
    let store = VectorStore::from_entries(vec![
        entry("x", "a", vec![1.0, 0.0]),
        entry("y", "a", vec![0.0, 1.0]),
        entry("z", "b", vec![0.8, 0.2]),
    ]);
    let matches = store.top_k_with_diagnostics(&[1.0, 0.0], None, 2, None);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].image_id, "x");
    assert_eq!(matches[1].image_id, "z");
}

#[test]
fn upsert_replaces_without_growing() {
    let mut store = VectorStore::from_entries(vec![entry("x", "a", vec![1.0, 0.0])]);
    store.upsert([("x".into(), "b".into(), vec![0.0, 1.0])]);
    assert_eq!(store.len(), 1);
    assert_eq!(store.top_k_with_diagnostics(&[0.0, 1.0], Some("b"), 1, None)[0].image_id, "x");
}

#[test]
fn removes_a_folder_without_touching_other_vectors() {
    let mut store = VectorStore::from_entries(vec![
        entry("x", "a", vec![1.0, 0.0]),
        entry("y", "b", vec![0.0, 1.0]),
    ]);
    store.remove_folder("a");
    assert_eq!(store.len(), 1);
    assert!(store.vector("x").is_none());
    assert!(store.vector("y").is_some());
}
