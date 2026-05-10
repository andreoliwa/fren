//! Slugify orchestration: combines `slug-preserve` with `fren`-specific
//! pre/post passes (CamelCase splitting, "at"-time-separator handling,
//! date detection).

mod camel_iso;
mod sentinel;

pub use camel_iso::{slugify_camel_iso, slugify_camel_iso_with_year};
// SENTINEL constant is reserved for future use. The current pipeline uses
// `_` directly because the date-format table is keyed off `_`. Kept around
// in case the pipeline ever needs to switch to a non-`_` internal sentinel
// (e.g. if user-chosen separator can be `_`).
#[allow(dead_code, unused_imports)]
pub(crate) use sentinel::SENTINEL;
