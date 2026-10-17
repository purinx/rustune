use crate::track::{EntryId, TrackId};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub id: EntryId,
    pub track_id: TrackId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueEffect {
    None,
    Play(QueueEntry),
    Stop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidQueue;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    entries: Vec<QueueEntry>,
    current: Option<EntryId>,
}
impl Queue {
    pub fn entries(&self) -> &[QueueEntry] {
        &self.entries
    }
    pub fn current(&self) -> Option<QueueEntry> {
        self.current
            .and_then(|id| self.entries.iter().find(|e| e.id == id).copied())
    }
    pub fn replace_and_select(
        &mut self,
        entries: Vec<QueueEntry>,
        selected: Option<EntryId>,
    ) -> Result<QueueEffect, InvalidQueue> {
        let unique: HashSet<_> = entries.iter().map(|e| e.id).collect();
        if unique.len() != entries.len() || selected.is_some_and(|id| !unique.contains(&id)) {
            return Err(InvalidQueue);
        }
        self.entries = entries;
        self.current = selected;
        Ok(self.current().map_or(QueueEffect::Stop, QueueEffect::Play))
    }
    pub fn append(&mut self, entry: QueueEntry) -> Result<QueueEffect, InvalidQueue> {
        if self.entries.iter().any(|e| e.id == entry.id) {
            return Err(InvalidQueue);
        }
        self.entries.push(entry);
        Ok(QueueEffect::None)
    }
    pub fn next_entry(&mut self) -> QueueEffect {
        let Some(index) = self
            .current
            .and_then(|id| self.entries.iter().position(|e| e.id == id))
        else {
            return QueueEffect::Stop;
        };
        self.current = self.entries.get(index + 1).map(|e| e.id);
        self.current().map_or(QueueEffect::Stop, QueueEffect::Play)
    }
    pub fn previous(&mut self) -> QueueEffect {
        let Some(index) = self
            .current
            .and_then(|id| self.entries.iter().position(|e| e.id == id))
        else {
            return QueueEffect::Stop;
        };
        self.current = Some(self.entries[index.saturating_sub(1)].id);
        QueueEffect::Play(self.current().expect("current entry exists"))
    }
    pub fn remove_track(&mut self, track: TrackId) -> QueueEffect {
        let removed_current = self.current().is_some_and(|e| e.track_id == track);
        let next = if removed_current {
            self.current
                .and_then(|id| self.entries.iter().position(|e| e.id == id))
                .and_then(|index| {
                    self.entries[index + 1..]
                        .iter()
                        .find(|e| e.track_id != track)
                })
                .map(|e| e.id)
        } else {
            self.current
        };
        self.entries.retain(|e| e.track_id != track);
        self.current = next;
        if removed_current {
            self.current().map_or(QueueEffect::Stop, QueueEffect::Play)
        } else {
            QueueEffect::None
        }
    }
}
