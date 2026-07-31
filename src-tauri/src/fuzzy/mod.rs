mod index;
mod color;
mod normalize;
mod ranking;

pub use index::FuzzyIndex;
pub use color::{ColorKind, dominance_requested, parse_colors, score_signature, signature_for_path};
pub use normalize::{fts_query, fts_query_or, normalize_query};
pub use ranking::{name_match_quality, reciprocal_rank, token_coverage};
