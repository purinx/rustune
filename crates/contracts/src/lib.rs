//! Public wire representations. Never serialize database rows or raw audio tags.

use serde::{Deserialize, Serialize};

/// Connectivity information for the development foundation, not library readiness.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
    pub stage: String,
}
