#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidTransition;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobEvent {
    Start,
    Complete,
    Fail,
    Interrupt,
}
pub fn transition_job(current: JobState, event: JobEvent) -> Result<JobState, InvalidTransition> {
    match (current, event) {
        (JobState::Queued, JobEvent::Start) => Ok(JobState::Running),
        (JobState::Queued | JobState::Running, JobEvent::Fail)
        | (JobState::Running, JobEvent::Interrupt) => Ok(JobState::Failed),
        (JobState::Running, JobEvent::Complete) => Ok(JobState::Completed),
        _ => Err(InvalidTransition),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Pending,
    Checking,
    Copying,
    Imported,
    Skipped,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemEvent {
    Check,
    Copy,
    Publish,
    Skip,
    Fail,
    Interrupt,
}
pub fn transition_item(
    current: ItemState,
    event: ItemEvent,
) -> Result<ItemState, InvalidTransition> {
    use ItemEvent::*;
    use ItemState::*;
    match (current, event) {
        (Pending, Check) => Ok(Checking),
        (Checking, Copy) => Ok(Copying),
        (Copying, Publish) => Ok(Imported),
        (Checking | Copying, Skip) => Ok(Skipped),
        (Pending | Checking | Copying, Fail | Interrupt) => Ok(Failed),
        _ => Err(InvalidTransition),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    total: Option<u64>,
    added: u64,
    skipped: u64,
    failed: u64,
}
impl Progress {
    pub fn new(
        total: Option<u64>,
        added: u64,
        skipped: u64,
        failed: u64,
    ) -> Result<Self, InvalidTransition> {
        let processed = added
            .checked_add(skipped)
            .and_then(|x| x.checked_add(failed))
            .ok_or(InvalidTransition)?;
        if total.is_some_and(|n| processed > n) {
            return Err(InvalidTransition);
        }
        Ok(Self {
            total,
            added,
            skipped,
            failed,
        })
    }
    pub fn total(self) -> Option<u64> {
        self.total
    }
    pub fn processed(self) -> u64 {
        self.added + self.skipped + self.failed
    }
    pub fn added(self) -> u64 {
        self.added
    }
    pub fn skipped(self) -> u64 {
        self.skipped
    }
    pub fn failed(self) -> u64 {
        self.failed
    }
}
