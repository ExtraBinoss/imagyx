mod files;
mod progress;
mod scan;
mod search;

pub use crate::vector_store::cosine_similarity;
pub use files::is_supported_image;
pub use scan::{index_folder, pending_assets};
pub use search::search;
