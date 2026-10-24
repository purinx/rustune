//! SQL専用の境界。すべてのメソッドは戻る前にtransactionを完了するため、
//! サービスがファイルコピー中に書き込みtransactionを保持し続けることはない。

use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, time::Duration};
use uuid::Uuid;

pub mod settings;

#[derive(Debug)]
pub enum DbError {
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
    Conflict,
    InvalidInput,
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Database(_) => "database operation failed",
            Self::Migration(_) => "database migration failed",
            Self::Conflict => "revision or state conflict",
            Self::InvalidInput => "invalid persistence input",
        })
    }
}
impl std::error::Error for DbError {}
impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

#[derive(Clone)]
pub struct Repository {
    pool: SqlitePool,
}
impl Repository {
    pub async fn open(path: &Path) -> Result<Self, DbError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
            pool.close().await;
            return Err(DbError::Migration(error));
        }
        Ok(Self { pool })
    }
    /// SQLを扱うサービスはpoolを共有するが、transactionをファイルI/Oへ渡さない。
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    pub async fn close(&self) {
        self.pool.close().await;
    }
    pub async fn library_revision(&self) -> Result<u64, DbError> {
        let value: i64 =
            sqlx::query_scalar("SELECT library_revision FROM library_state WHERE singleton=1")
                .fetch_one(&self.pool)
                .await?;
        Ok(value as u64)
    }
    pub async fn create_playlist(&self, id: Uuid, name: &str) -> Result<(), DbError> {
        if id.is_nil() || !(1..=120).contains(&name.trim().chars().count()) {
            return Err(DbError::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO playlists(id,name) VALUES(?,?)")
            .bind(id.to_string())
            .bind(name.trim())
            .execute(&mut *tx)
            .await?;
        bump_library(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn rename_playlist(
        &self,
        id: Uuid,
        expected: u64,
        name: &str,
    ) -> Result<u64, DbError> {
        if !(1..=120).contains(&name.trim().chars().count()) {
            return Err(DbError::InvalidInput);
        }
        let expected = revision(expected)?;
        let mut tx = self.pool.begin().await?;
        let updated=sqlx::query("UPDATE playlists SET name=?,revision=revision+1 WHERE id=? AND revision=? AND revision<9223372036854775807")
            .bind(name.trim()).bind(id.to_string()).bind(expected).execute(&mut *tx).await?;
        if updated.rows_affected() != 1 {
            return Err(DbError::Conflict);
        }
        bump_library(&mut tx).await?;
        tx.commit().await?;
        Ok((expected + 1) as u64)
    }
    /// 要求を積み増さず、未完了のjobがあればそれを再利用する。ここではワーカーを起動しない。
    pub async fn reserve_import(
        &self,
        id: Uuid,
        trigger: &str,
        created_at: &str,
    ) -> Result<Uuid, DbError> {
        if id.is_nil()
            || !["manual", "startup", "scheduled"].contains(&trigger)
            || created_at.is_empty()
        {
            return Err(DbError::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO import_jobs(id,trigger,state,phase,settings_revision,created_at) SELECT ?,?,'queued','enumerating',settings_revision,? FROM settings WHERE singleton=1 ON CONFLICT DO NOTHING")
            .bind(id.to_string()).bind(trigger).bind(created_at).execute(&mut *tx).await?;
        let active = sqlx::query("SELECT id FROM import_jobs WHERE state IN ('queued','running')")
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(DbError::InvalidInput)?;
        let active =
            Uuid::parse_str(active.try_get::<&str, _>("id")?).map_err(|_| DbError::InvalidInput)?;
        tx.commit().await?;
        Ok(active)
    }
}
fn revision(value: u64) -> Result<i64, DbError> {
    i64::try_from(value).map_err(|_| DbError::InvalidInput)
}
async fn bump_library(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<(), DbError> {
    let result=sqlx::query("UPDATE library_state SET library_revision=library_revision+1 WHERE singleton=1 AND library_revision<9223372036854775807").execute(&mut **tx).await?;
    if result.rows_affected() != 1 {
        return Err(DbError::Conflict);
    }
    Ok(())
}
