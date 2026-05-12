pub mod data_loader;
pub mod normalize;
pub mod romaji;
pub mod storage;
pub mod validator;

pub use data_loader::DataLoader;
pub use storage::Storage;
pub use validator::{find_prefix_conflicts, format_conflict};
// PrefixConflict stays accessible via `io::validator::PrefixConflict` for the
// build-time linter binary planned in #60; not re-exported at this level
// until a caller in the bin actually constructs it.
