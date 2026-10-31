CREATE TABLE artworks (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    origin TEXT NOT NULL CHECK(origin IN ('embedded','enriched')),
    path TEXT NOT NULL UNIQUE,
    width INTEGER NOT NULL CHECK(width>0), height INTEGER NOT NULL CHECK(height>0),
    digest BLOB NOT NULL CHECK(length(digest)=32), store_url TEXT
) STRICT;

CREATE TABLE tracks (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    state TEXT NOT NULL CHECK(state IN ('staging','ready','deleting','deleted','error')),
    media_path TEXT UNIQUE,
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
    file_digest BLOB CHECK(file_digest IS NULL OR length(file_digest)=32),
    audio_digest BLOB CHECK(audio_digest IS NULL OR length(audio_digest)=32),
    audio_digest_scheme TEXT,
    store_track_id INTEGER CHECK(store_track_id IS NULL OR store_track_id>0),
    size_bytes INTEGER CHECK(size_bytes IS NULL OR size_bytes>0),
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms>0),
    artwork_id TEXT REFERENCES artworks(id),
    created_at TEXT NOT NULL,
    error_code TEXT,
    CHECK(state!='ready' OR (media_path IS NOT NULL AND file_digest IS NOT NULL
        AND audio_digest IS NOT NULL AND audio_digest_scheme IS NOT NULL
        AND size_bytes IS NOT NULL AND duration_ms IS NOT NULL))
) STRICT;
CREATE INDEX tracks_state ON tracks(state);

CREATE TABLE track_metadata (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK(schema_version>0),
    imported_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(imported_json)),
    enriched_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(enriched_json)),
    manual_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(manual_json)),
    title TEXT NOT NULL, artist TEXT NOT NULL, album TEXT NOT NULL,
    album_artist TEXT, disc_number INTEGER CHECK(disc_number BETWEEN 1 AND 999),
    track_number INTEGER CHECK(track_number BETWEEN 1 AND 999),
    search_text TEXT NOT NULL, album_key TEXT NOT NULL, artist_key TEXT NOT NULL,
    store_group_released INTEGER NOT NULL DEFAULT 0 CHECK(store_group_released IN (0,1))
) STRICT;
CREATE INDEX metadata_album ON track_metadata(album_key);
CREATE INDEX metadata_artist ON track_metadata(artist_key);

CREATE TABLE source_roots (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    absolute_path_bytes BLOB NOT NULL,
    display_name TEXT NOT NULL CHECK(length(display_name)>0),
    active INTEGER NOT NULL CHECK(active IN (0,1))
) STRICT;
CREATE UNIQUE INDEX one_active_source_root ON source_roots(active) WHERE active=1;
CREATE TABLE sources (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    root_id TEXT NOT NULL REFERENCES source_roots(id),
    relative_path_bytes BLOB NOT NULL, display_path TEXT NOT NULL,
    track_id TEXT REFERENCES tracks(id),
    device INTEGER, inode INTEGER, size_bytes INTEGER CHECK(size_bytes IS NULL OR size_bytes>=0),
    mtime_seconds INTEGER, mtime_nanos INTEGER CHECK(mtime_nanos BETWEEN 0 AND 999999999),
    parser_version TEXT NOT NULL, outcome TEXT NOT NULL,
    imported_at TEXT, last_seen_at TEXT NOT NULL,
    UNIQUE(root_id,relative_path_bytes)
) STRICT;
CREATE INDEX sources_track ON sources(track_id);
CREATE TABLE identities (
    scheme TEXT NOT NULL, value BLOB NOT NULL,
    track_id TEXT NOT NULL REFERENCES tracks(id),
    PRIMARY KEY(scheme,value)
) STRICT;
CREATE INDEX identities_track ON identities(track_id);
CREATE TABLE exclusions (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id), excluded_at TEXT NOT NULL
) STRICT;

CREATE TABLE import_jobs (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    trigger TEXT NOT NULL CHECK(trigger IN ('manual','startup','scheduled')),
    state TEXT NOT NULL CHECK(state IN ('queued','running','completed','failed')),
    phase TEXT NOT NULL CHECK(phase IN ('enumerating','processing')),
    total INTEGER CHECK(total IS NULL OR total>=0),
    processed INTEGER NOT NULL DEFAULT 0 CHECK(processed>=0),
    added INTEGER NOT NULL DEFAULT 0 CHECK(added>=0),
    skipped INTEGER NOT NULL DEFAULT 0 CHECK(skipped>=0),
    failed INTEGER NOT NULL DEFAULT 0 CHECK(failed>=0),
    settings_revision INTEGER NOT NULL CHECK(settings_revision>=0),
    created_at TEXT NOT NULL, started_at TEXT, finished_at TEXT, reason TEXT,
    CHECK(processed=added+skipped+failed), CHECK(total IS NULL OR processed<=total),
    CHECK(phase!='processing' OR total IS NOT NULL),
    CHECK((state IN ('completed','failed')) = (finished_at IS NOT NULL))
) STRICT;
CREATE UNIQUE INDEX one_unfinished_import ON import_jobs((1)) WHERE state IN ('queued','running');
CREATE INDEX import_jobs_created ON import_jobs(created_at);
CREATE TABLE import_items (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    source_id TEXT REFERENCES sources(id),
    root_id TEXT NOT NULL REFERENCES source_roots(id),
    relative_path_bytes BLOB NOT NULL, display_path TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending','checking','copying','imported','skipped','failed')),
    reason TEXT, retryable INTEGER NOT NULL DEFAULT 0 CHECK(retryable IN (0,1)),
    track_id TEXT REFERENCES tracks(id),
    CHECK(state!='imported' OR track_id IS NOT NULL)
) STRICT;
CREATE INDEX import_items_job_state ON import_items(job_id,state);

CREATE TABLE playlists (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0)
) STRICT;
CREATE TABLE playlist_items (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=36),
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id),
    position INTEGER NOT NULL CHECK(position>=0),
    UNIQUE(playlist_id,position)
) STRICT;
CREATE INDEX playlist_items_track ON playlist_items(track_id);
CREATE TABLE favorites (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id)
) STRICT;
CREATE TABLE library_state (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton=1),
    library_revision INTEGER NOT NULL CHECK(library_revision>=0)
) STRICT;
INSERT INTO library_state(singleton,library_revision) VALUES(1,0);
CREATE TABLE settings (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton=1),
    source_root_id TEXT NOT NULL REFERENCES source_roots(id),
    auto_import INTEGER NOT NULL DEFAULT 1 CHECK(auto_import IN (0,1)),
    interval_seconds INTEGER NOT NULL DEFAULT 60 CHECK(interval_seconds BETWEEN 30 AND 3600),
    max_file_bytes INTEGER NOT NULL DEFAULT 209715200 CHECK(max_file_bytes BETWEEN 1048576 AND 2147483648),
    reserve_free_bytes INTEGER NOT NULL DEFAULT 1073741824 CHECK(reserve_free_bytes>=1073741824),
    storefront TEXT NOT NULL DEFAULT 'jp' CHECK(length(storefront)=2 AND storefront GLOB '[a-z][a-z]'),
    settings_revision INTEGER NOT NULL DEFAULT 0 CHECK(settings_revision>=0)
) STRICT;

-- 確定処理の復旧用に永続化するデータ。内部パスは公開APIの値にしない。
CREATE TABLE staging_manifests (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id),
    staging_path TEXT NOT NULL UNIQUE, final_path TEXT NOT NULL UNIQUE,
    manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json)),
    created_at TEXT NOT NULL
) STRICT;
