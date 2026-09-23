//! Unit tests for the `ai_queries` persistence layer in [`super`].
//!
//! Covers the FIFO eviction cap added to [`super::upsert_ai_query`] and the empty-input filter
//! that drives the persistence skip in `handle_ai_history_event`.
