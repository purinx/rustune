//! storage・artworkテスト共通の障害注入I/O。
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use rustune_server::storage::{OsIo, StorageIo};

/// 各操作を個別に失敗させられるI/O。既定ではOSの動作をそのまま行う。
#[derive(Default)]
pub struct FaultyIo {
    /// 累計でこのバイト数を超えて書き込もうとしたら失敗させる。
    pub fail_write_after: Mutex<Option<u64>>,
    written: AtomicU64,
    pub fail_sync_file: AtomicBool,
    pub fail_sync_dir: AtomicBool,
    /// 残り回数だけrenameを成功させ、その後は失敗させる。
    pub renames_before_failure: Mutex<Option<usize>>,
    pub fail_remove: AtomicBool,
    /// 空き容量の応答列。最後の値を以後も返す。空なら実際の値。
    pub space: Mutex<VecDeque<u64>>,
    pub space_calls: AtomicUsize,
}

fn injected(what: &str) -> io::Error {
    io::Error::other(format!("injected {what} failure"))
}

impl FaultyIo {
    pub fn with_space(values: &[u64]) -> Self {
        let io = Self::default();
        io.space.lock().unwrap().extend(values.iter().copied());
        io
    }
}

impl StorageIo for FaultyIo {
    fn available_space(&self, dir: &Path) -> io::Result<u64> {
        self.space_calls.fetch_add(1, Ordering::SeqCst);
        let mut space = self.space.lock().unwrap();
        match space.len() {
            0 => OsIo.available_space(dir),
            1 => Ok(space[0]),
            _ => Ok(space.pop_front().unwrap()),
        }
    }

    fn write_all(&self, file: &mut File, buf: &[u8]) -> io::Result<()> {
        if let Some(limit) = *self.fail_write_after.lock().unwrap() {
            let done = self.written.load(Ordering::SeqCst);
            if done + buf.len() as u64 > limit {
                // 途中まで書いてから失敗する部分書き込みを再現する。
                let partial = limit.saturating_sub(done) as usize;
                OsIo.write_all(file, &buf[..partial])?;
                self.written.fetch_add(partial as u64, Ordering::SeqCst);
                return Err(injected("write"));
            }
        }
        self.written.fetch_add(buf.len() as u64, Ordering::SeqCst);
        OsIo.write_all(file, buf)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        if self.fail_sync_file.load(Ordering::SeqCst) {
            return Err(injected("sync"));
        }
        OsIo.sync_file(file)
    }

    fn rename_noreplace(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut remaining = self.renames_before_failure.lock().unwrap();
        if let Some(count) = remaining.as_mut() {
            if *count == 0 {
                return Err(injected("rename"));
            }
            *count -= 1;
        }
        OsIo.rename_noreplace(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        if self.fail_sync_dir.load(Ordering::SeqCst) {
            return Err(injected("directory sync"));
        }
        OsIo.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        if self.fail_remove.load(Ordering::SeqCst) {
            return Err(injected("remove"));
        }
        OsIo.remove_file(path)
    }
}
