//! Rustune's domain layer. No HTTP, storage, UI, or operating-system dependencies.
//! IDs are supplied by the caller: this crate never requests OS randomness.

pub mod identity;
pub mod import_state;
pub mod metadata;
pub mod queue;
pub mod track;
