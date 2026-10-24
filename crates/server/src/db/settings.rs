use super::{DbError, Repository, bump_library, revision};
use sqlx::{ConnectOptions, Row, sqlite::SqliteConnectOptions};
use std::path::Path;
use uuid::Uuid;

/// 既存DBに保存された取り込み元のパス（OSのバイト列）を、読み取り専用で取り出す。
///
/// 起動時に、保存先の準備（ディレクトリ作成・権限変更）とmigrationを行う前に、
/// 保存済みの取り込み元と保存先が重なっていないかを確かめるために使う（`bootstrap`）。
/// DBの作成・migration・設定の書き込みはしない。DBファイルが無いとき、
/// またはmigration前で設定テーブルが無いときは`None`を返す。
///
/// WALモードのDBを読み取り専用で開くとき、SQLiteは`-wal`・`-shm`ファイルが無ければ
/// DBと同じディレクトリに作ることがある。DBファイル自体がすでにそこにあるので、
/// 新しい場所へ書き込むことにはならない。
pub async fn stored_source_path(db_path: &Path) -> Result<Option<Vec<u8>>, DbError> {
    // 有無を調べられない場合（data_dirがファイルなど）も「既存DBなし」とし、
    // 保存先の問題としてbootstrapの`prepare_storage`に報告させる。
    if !db_path.try_exists().unwrap_or(false) {
        return Ok(None);
    }
    let mut connection = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .connect()
        .await?;
    let result = read_stored_source_path(&mut connection).await;
    let _ = sqlx::Connection::close(connection).await;
    result
}
async fn read_stored_source_path(
    connection: &mut sqlx::SqliteConnection,
) -> Result<Option<Vec<u8>>, DbError> {
    let has_settings: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings')",
    )
    .fetch_one(&mut *connection)
    .await?;
    if !has_settings {
        return Ok(None);
    }
    Ok(sqlx::query_scalar("SELECT r.absolute_path_bytes FROM settings s JOIN source_roots r ON r.id=s.source_root_id WHERE s.singleton=1")
        .fetch_optional(&mut *connection)
        .await?)
}

/// サーバー内部の設定。パスは表示用の非可逆文字列ではなくOSのバイト列で保持する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySettings {
    pub source_root_id: Uuid,
    pub source_path_bytes: Vec<u8>,
    pub source_display_name: String,
    pub auto_import: bool,
    pub interval_seconds: u32,
    pub max_file_bytes: u64,
    pub reserve_free_bytes: u64,
    pub storefront: String,
    pub revision: u64,
}
impl LibrarySettings {
    pub fn defaults(
        source_root_id: Uuid,
        source_path_bytes: Vec<u8>,
        source_display_name: String,
    ) -> Self {
        Self {
            source_root_id,
            source_path_bytes,
            source_display_name,
            auto_import: true,
            interval_seconds: 60,
            max_file_bytes: 209715200,
            reserve_free_bytes: 1073741824,
            storefront: "jp".into(),
            revision: 0,
        }
    }
    pub fn validate(&self) -> Result<(), DbError> {
        if self.source_root_id.is_nil()
            || self.source_path_bytes.first() != Some(&b'/')
            || self.source_path_bytes.contains(&0)
            || self.source_display_name.trim().is_empty()
            || !(30..=3600).contains(&self.interval_seconds)
            || !(1048576..=2147483648).contains(&self.max_file_bytes)
            || self.reserve_free_bytes < 1073741824
            || self.reserve_free_bytes > i64::MAX as u64
            || self.storefront.len() != 2
            || !self.storefront.bytes().all(|c| c.is_ascii_lowercase())
        {
            return Err(DbError::InvalidInput);
        }
        revision(self.revision)?;
        Ok(())
    }
}
impl Repository {
    /// 新規DBのときだけ初期化する。既存の設定は上書きしない。
    pub async fn initialize_settings(&self, defaults: &LibrarySettings) -> Result<(), DbError> {
        defaults.validate()?;
        let mut tx = self.pool.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM settings)")
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            sqlx::query("INSERT INTO source_roots(id,absolute_path_bytes,display_name,active) VALUES(?,?,?,1)").bind(defaults.source_root_id.to_string()).bind(&defaults.source_path_bytes).bind(&defaults.source_display_name).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO settings(singleton,source_root_id,auto_import,interval_seconds,max_file_bytes,reserve_free_bytes,storefront) VALUES(1,?,?,?,?,?,?)")
                .bind(defaults.source_root_id.to_string()).bind(defaults.auto_import).bind(i64::from(defaults.interval_seconds)).bind(defaults.max_file_bytes as i64).bind(defaults.reserve_free_bytes as i64).bind(&defaults.storefront).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn settings(&self) -> Result<LibrarySettings, DbError> {
        let row=sqlx::query("SELECT s.*,r.absolute_path_bytes,r.display_name FROM settings s JOIN source_roots r ON r.id=s.source_root_id WHERE s.singleton=1").fetch_one(&self.pool).await?;
        Ok(LibrarySettings {
            source_root_id: Uuid::parse_str(row.try_get("source_root_id")?)
                .map_err(|_| DbError::InvalidInput)?,
            source_path_bytes: row.try_get("absolute_path_bytes")?,
            source_display_name: row.try_get("display_name")?,
            auto_import: row.try_get("auto_import")?,
            interval_seconds: row.try_get::<i64, _>("interval_seconds")? as u32,
            max_file_bytes: row.try_get::<i64, _>("max_file_bytes")? as u64,
            reserve_free_bytes: row.try_get::<i64, _>("reserve_free_bytes")? as u64,
            storefront: row.try_get("storefront")?,
            revision: row.try_get::<i64, _>("settings_revision")? as u64,
        })
    }
    /// root変更には新しいroot UUIDが必要で、過去のrootはすべて保持する。
    /// パスの重なり・権限の確認はこの呼び出しの前にconfig/sourceで行う。
    pub async fn update_settings(
        &self,
        expected: u64,
        next: &LibrarySettings,
    ) -> Result<u64, DbError> {
        next.validate()?;
        let expected = revision(expected)?;
        let mut tx = self.pool.begin().await?;
        // 可変状態を読む前にSQLiteの書き込みロックを取得し、設定の並行更新時に
        // 読み取りから書き込みへの遅延昇格で競合が起きないようにする。
        let acquired=sqlx::query("UPDATE settings SET settings_revision=settings_revision WHERE singleton=1 AND settings_revision=? AND settings_revision<9223372036854775807").bind(expected).execute(&mut *tx).await?;
        if acquired.rows_affected() != 1 {
            return Err(DbError::Conflict);
        }
        let row=sqlx::query("SELECT r.id,r.absolute_path_bytes FROM settings s JOIN source_roots r ON r.id=s.source_root_id WHERE singleton=1").fetch_one(&mut *tx).await?;
        let old_id: String = row.try_get("id")?;
        let old_path: Vec<u8> = row.try_get("absolute_path_bytes")?;
        let changing_root = old_path != next.source_path_bytes;
        if changing_root {
            if old_id == next.source_root_id.to_string() {
                return Err(DbError::InvalidInput);
            }
            let busy: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM import_jobs WHERE state IN ('queued','running'))",
            )
            .fetch_one(&mut *tx)
            .await?;
            if busy {
                return Err(DbError::Conflict);
            }
            sqlx::query("UPDATE source_roots SET active=0 WHERE active=1")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO source_roots(id,absolute_path_bytes,display_name,active) VALUES(?,?,?,1)").bind(next.source_root_id.to_string()).bind(&next.source_path_bytes).bind(&next.source_display_name).execute(&mut *tx).await?;
        } else if old_id != next.source_root_id.to_string() {
            return Err(DbError::InvalidInput);
        }
        sqlx::query("UPDATE settings SET source_root_id=?,auto_import=?,interval_seconds=?,max_file_bytes=?,reserve_free_bytes=?,storefront=?,settings_revision=settings_revision+1 WHERE singleton=1")
            .bind(next.source_root_id.to_string()).bind(next.auto_import).bind(i64::from(next.interval_seconds)).bind(next.max_file_bytes as i64).bind(next.reserve_free_bytes as i64).bind(&next.storefront).execute(&mut *tx).await?;
        bump_library(&mut tx).await?;
        tx.commit().await?;
        Ok((expected + 1) as u64)
    }
}
