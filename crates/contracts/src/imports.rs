use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTrigger {
    Manual,
    Startup,
    Scheduled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Enumerating,
    Processing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Pending,
    Checking,
    Copying,
    Imported,
    Skipped,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub trigger: ImportTrigger,
    pub status: JobStatus,
    pub phase: ImportPhase,
    pub total: Option<u64>,
    pub processed: u64,
    pub added: u64,
    pub skipped: u64,
    pub failed: u64,
    pub reason_counts: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportItem {
    pub id: Uuid,
    pub source_root_name: String,
    pub relative_path: String,
    pub status: ItemStatus,
    pub reason: Option<String>,
    pub retryable: bool,
    pub track_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportAccepted {
    pub job_id: Uuid,
}
