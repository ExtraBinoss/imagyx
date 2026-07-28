mod files;
mod progress;
mod scan;
mod search;

pub use crate::vector_store::cosine_similarity;
pub(crate) use files::prepare_asset;
pub use scan::{index_folder, pending_assets};
pub use search::{search, search_page_with_diagnostics, search_with_diagnostics};
