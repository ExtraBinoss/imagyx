mod normalize;
mod ranking;

pub use normalize::{fts_query, normalize_query};
pub use ranking::{exact_name_bonus, reciprocal_rank};
