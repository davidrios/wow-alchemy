//! Shared utilities for the wow-alchemy CLI

#[cfg(any(
    feature = "dbc",
    feature = "blp",
    feature = "m2",
    feature = "wmo",
    feature = "adt",
    feature = "wdt",
    feature = "wdl",
    test
))]
pub mod progress;

#[cfg(any(
    feature = "dbc",
    feature = "blp",
    feature = "m2",
    feature = "wmo",
    feature = "adt",
    feature = "wdt",
    feature = "wdl",
    test
))]
pub mod table;

#[cfg(any(
    feature = "dbc",
    feature = "blp",
    feature = "m2",
    feature = "wmo",
    feature = "adt",
    feature = "wdt",
    feature = "wdl",
))]
pub mod tree;

// Re-export utilities only when actually used by commands

#[cfg(any(
    feature = "m2",
    feature = "wmo",
    feature = "adt",
    feature = "wdt",
    feature = "wdl"
))]
pub use tree::*;
