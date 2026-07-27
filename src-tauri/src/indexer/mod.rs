mod files;
mod progress;
mod scan;
mod search;

pub(crate) use files::prepare_asset;
pub use crate::vector_store::cosine_similarity;
pub use scan::{index_folder, pending_assets};
pub use search::search;
