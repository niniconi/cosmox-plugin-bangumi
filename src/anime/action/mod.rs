//! Metadata enrichment for the Bangumi media type.
//!
//! This module owns the two functional domains of the plugin:
//!
//! - `enrich`: cast / staff / episode / cover annotation of tree nodes.
//! - `rebuild`: reads the raw metadata tree via `MetadataView` and
//!   rewrites it with Bangumi API data.
//!
//! The module path `anime::action` is part of the public contract
//! (consumed by `lib.rs` via `crate::anime::action::rebuild_metadata_tree`),
//! so the re-exports below keep those absolute paths stable.

pub(crate) mod enrich;
pub(crate) mod rebuild;

// Public contract re-exports (paths must stay stable).
pub use rebuild::rebuild_metadata_tree;
