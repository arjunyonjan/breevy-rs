//! Breevy-compatible text expander core.
//!
//! The `.bvy` format is a UTF-8 XML property-list with one `<a>` element per
//! abbreviation. This crate parses those records and implements the observed
//! trigger/match/expand semantics of the original application.

pub mod bvy;
pub mod engine;
pub mod macros;

pub use bvy::{parse_bvy, remove_bvy_record, update_bvy_record, write_bvy, Snippet};
pub use engine::{Expansion, Expander};
pub use macros::expand_macros;

#[cfg(feature = "editor")]
pub mod gui;
