use crate::error::Result;
use crate::library::metadata::TrackMeta;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Track {
    pub id: i64,
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub duration: Option<f64>,
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub file_size: i64,
    pub modified_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        self.conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);",
        )?;

        let version: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if version < 1 {
            self.conn.execute_batch(include_str!("schema.sql"))?;
            self.conn
                .execute("INSERT INTO schema_version VALUES (?1)", params![1])?;
        }
        Ok(())
    }

    // ── Tracks ────────────────────────────────────────────────────────────────

    /// Upsert a track. Returns the row id.
    pub fn upsert_track(
        &self,
        path: &str,
        meta: &TrackMeta,
        file_size: i64,
        modified_at: i64,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO tracks (path, title, artist, album, genre, duration, codec,
                                 sample_rate, bit_depth, file_size, modified_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT(path) DO UPDATE SET
               title=excluded.title, artist=excluded.artist, album=excluded.album,
               genre=excluded.genre, duration=excluded.duration, codec=excluded.codec,
               sample_rate=excluded.sample_rate, bit_depth=excluded.bit_depth,
               file_size=excluded.file_size, modified_at=excluded.modified_at",
            params![
                path,
                meta.title,
                meta.artist,
                meta.album,
                meta.genre,
                meta.duration,
                meta.codec,
                meta.sample_rate,
                meta.bit_depth.map(|b| b as i64),
                file_size,
                modified_at,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Check if a track needs re-scanning (file size or mtime changed).
    pub fn track_needs_update(&self, path: &str, file_size: i64, modified_at: i64) -> Result<bool> {
        let result: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT file_size, modified_at FROM tracks WHERE path = ?1",
                params![path],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        match result {
            None => Ok(true), // not in DB
            Some((sz, mt)) => Ok(sz != file_size || mt != modified_at),
        }
    }

    pub fn get_all_tracks(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, genre, duration, codec,
                    sample_rate, bit_depth, file_size, modified_at
             FROM tracks ORDER BY artist, album, title",
        )?;
        let tracks = stmt
            .query_map([], |r| {
                Ok(Track {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    album: r.get(4)?,
                    genre: r.get(5)?,
                    duration: r.get(6)?,
                    codec: r.get(7)?,
                    sample_rate: r.get(8)?,
                    bit_depth: r.get(9).ok().flatten(),
                    file_size: r.get(10)?,
                    modified_at: r.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tracks)
    }

    pub fn search_tracks(&self, pattern: &str) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM tracks WHERE
             lower(title) LIKE ?1 OR lower(artist) LIKE ?1 OR lower(album) LIKE ?1 OR lower(path) LIKE ?1
             ORDER BY artist, album, title",
        )?;
        let ids = stmt
            .query_map(params![pattern], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    pub fn get_track_by_id(&self, id: i64) -> Result<Option<Track>> {
        match self.conn.query_row(
            "SELECT id, path, title, artist, album, genre, duration, codec,
                    sample_rate, bit_depth, file_size, modified_at
             FROM tracks WHERE id = ?1",
            params![id],
            |r| {
                Ok(Track {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    album: r.get(4)?,
                    genre: r.get(5)?,
                    duration: r.get(6)?,
                    codec: r.get(7)?,
                    sample_rate: r.get(8)?,
                    bit_depth: r.get(9).ok().flatten(),
                    file_size: r.get(10)?,
                    modified_at: r.get(11)?,
                })
            },
        ) {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ── Playlists ─────────────────────────────────────────────────────────────

    pub fn create_playlist(&self, name: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT OR IGNORE INTO playlists (name) VALUES (?1)",
            params![name],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM playlists WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at FROM playlists ORDER BY name")?;
        let pl = stmt
            .query_map([], |r| {
                Ok(Playlist {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(pl)
    }

    pub fn add_track_to_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
        position: i64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, position],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.genre, t.duration,
                    t.codec, t.sample_rate, t.bit_depth, t.file_size, t.modified_at
             FROM tracks t
             JOIN playlist_tracks pt ON pt.track_id = t.id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )?;
        let tracks = stmt
            .query_map(params![playlist_id], |r| {
                Ok(Track {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    album: r.get(4)?,
                    genre: r.get(5)?,
                    duration: r.get(6)?,
                    codec: r.get(7)?,
                    sample_rate: r.get(8)?,
                    bit_depth: r.get(9).ok().flatten(),
                    file_size: r.get(10)?,
                    modified_at: r.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tracks)
    }

    /// Delete tracks whose files no longer exist on disk.
    /// Returns the number of pruned tracks.
    pub fn prune_missing_tracks(&self) -> Result<usize> {
        let mut stmt = self.conn.prepare("SELECT id, path FROM tracks")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut removed = 0;
        for (id, path) in rows {
            if !std::path::Path::new(&path).exists() {
                self.conn
                    .execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    // ── History ───────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn record_play(&self, track_id: i64, duration_played: f64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO playback_history (track_id, duration_played) VALUES (?1, ?2)",
            params![track_id, duration_played],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::metadata::TrackMeta;

    #[test]
    fn open_in_memory_and_migrate() {
        let db = Database::open_in_memory().unwrap();
        db.migrate().unwrap(); // idempotent
    }

    #[test]
    fn schema_version_is_1() {
        let db = Database::open_in_memory().unwrap();
        let v: i64 = db
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn upsert_and_retrieve_track() {
        let db = Database::open_in_memory().unwrap();
        let meta = TrackMeta {
            title: Some("Test Track".into()),
            artist: Some("Artist".into()),
            album: Some("Album".into()),
            duration: Some(180.0),
            ..Default::default()
        };
        let id = db
            .upsert_track("/music/test.mp3", &meta, 1024, 1700000000)
            .unwrap();
        let track = db.get_track_by_id(id).unwrap().unwrap();
        assert_eq!(track.title.as_deref(), Some("Test Track"));
        assert_eq!(track.artist.as_deref(), Some("Artist"));
    }

    #[test]
    fn track_needs_update_when_missing() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.track_needs_update("/music/new.mp3", 100, 200).unwrap());
    }

    #[test]
    fn track_no_update_when_same() {
        let db = Database::open_in_memory().unwrap();
        let meta = TrackMeta::default();
        db.upsert_track("/music/x.mp3", &meta, 500, 999).unwrap();
        assert!(!db.track_needs_update("/music/x.mp3", 500, 999).unwrap());
        assert!(db.track_needs_update("/music/x.mp3", 501, 999).unwrap());
    }

    #[test]
    fn search_tracks() {
        let db = Database::open_in_memory().unwrap();
        let meta = TrackMeta {
            title: Some("Blue Train".into()),
            artist: Some("Coltrane".into()),
            ..Default::default()
        };
        db.upsert_track("/music/blue.flac", &meta, 100, 100)
            .unwrap();
        let ids = db.search_tracks("%coltrane%").unwrap();
        assert_eq!(ids.len(), 1);
        let ids_none = db.search_tracks("%zzznotfound%").unwrap();
        assert!(ids_none.is_empty());
    }

    #[test]
    fn playlist_crud() {
        let db = Database::open_in_memory().unwrap();
        let pl_id = db.create_playlist("Favorites").unwrap();
        let meta = TrackMeta {
            title: Some("Song".into()),
            ..Default::default()
        };
        let t_id = db.upsert_track("/music/s.mp3", &meta, 10, 10).unwrap();
        db.add_track_to_playlist(pl_id, t_id, 0).unwrap();
        let tracks = db.get_playlist_tracks(pl_id).unwrap();
        assert_eq!(tracks.len(), 1);
        let pls = db.list_playlists().unwrap();
        assert_eq!(pls.len(), 1);
        assert_eq!(pls[0].name, "Favorites");

        db.delete_playlist(pl_id).unwrap();
        assert_eq!(db.list_playlists().unwrap().len(), 0);
        assert_eq!(db.get_playlist_tracks(pl_id).unwrap().len(), 0);
    }

    #[test]
    fn prune_missing_tracks_removes_nonexistent_files() {
        let db = Database::open_in_memory().unwrap();
        let meta = TrackMeta::default();
        db.upsert_track("/nonexistent/file/path/here.mp3", &meta, 100, 100)
            .unwrap();
        assert_eq!(db.get_all_tracks().unwrap().len(), 1);
        let pruned = db.prune_missing_tracks().unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(db.get_all_tracks().unwrap().len(), 0);
    }
}
