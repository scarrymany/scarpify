//! Persistent library: liked tracks, recently played, imported and user-made playlists.
//!
//! SQLite (bundled, WAL) keeps every write durable without size limits. Track metadata is
//! stored once in `tracks` and referenced by key, so a song liked and listed in three
//! playlists is a single row.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, Provider, Track};

/// Bump together with a new step in [`migrate`].
const SCHEMA_VERSION: i64 = 1;
const RECENT_LIMIT: i64 = 24;

const SCHEMA_V1: &str = "
    CREATE TABLE tracks (
        key  TEXT PRIMARY KEY,
        data TEXT NOT NULL
    );
    CREATE TABLE liked (
        track_key TEXT PRIMARY KEY REFERENCES tracks(key),
        added_at  INTEGER NOT NULL
    );
    CREATE TABLE recent (
        track_key TEXT PRIMARY KEY REFERENCES tracks(key),
        played_at INTEGER NOT NULL
    );
    CREATE TABLE collections (
        id             TEXT PRIMARY KEY,
        origin         TEXT NOT NULL,
        kind           TEXT NOT NULL,
        name           TEXT NOT NULL,
        owner          TEXT,
        artwork        TEXT,
        custom_artwork TEXT,
        provider       TEXT,
        source         TEXT UNIQUE,
        created_at     INTEGER NOT NULL,
        pinned         INTEGER NOT NULL DEFAULT 0,
        sort_order     INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE collection_tracks (
        collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
        position      INTEGER NOT NULL,
        track_key     TEXT NOT NULL REFERENCES tracks(key),
        PRIMARY KEY (collection_id, position)
    );
";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// Imported by link from a service.
    Import,
    /// Created and edited in SCARPIFY.
    User,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCollection {
    pub id: String,
    pub origin: Origin,
    pub kind: CollectionKind,
    pub name: String,
    pub owner: Option<String>,
    pub artwork: Option<String>,
    pub custom_artwork: Option<String>,
    pub provider: Option<Provider>,
    pub source: Option<String>,
    pub created_at: i64,
    pub pinned: bool,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub liked: Vec<Track>,
    pub recent: Vec<Track>,
    pub collections: Vec<SavedCollection>,
}

/// Library as kept in browser storage by builds before SQLite.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyLibrary {
    #[serde(default)]
    pub liked: Vec<Track>,
    #[serde(default)]
    pub recent: Vec<Track>,
    #[serde(default)]
    pub collections: Vec<LegacyCollection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCollection {
    pub id: String,
    pub source: String,
    pub imported_at: i64,
    #[serde(flatten)]
    pub collection: Collection,
}

pub struct Library {
    db: Mutex<Connection>,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.pragma_update(None, "journal_mode", "WAL")?;
        db.pragma_update(None, "foreign_keys", true)?;
        migrate(&db)?;
        Ok(Self { db: Mutex::new(db) })
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self> {
        let db = Connection::open_in_memory()?;
        db.pragma_update(None, "foreign_keys", true)?;
        migrate(&db)?;
        Ok(Self { db: Mutex::new(db) })
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let db = self.lock();
        let liked = tracks_by(&db, "SELECT t.data FROM liked l JOIN tracks t ON t.key = l.track_key ORDER BY l.added_at DESC")?;
        let recent = tracks_by(&db, "SELECT t.data FROM recent r JOIN tracks t ON t.key = r.track_key ORDER BY r.played_at DESC")?;

        let mut statement = db.prepare(
            "SELECT id, origin, kind, name, owner, artwork, custom_artwork, provider, source, created_at, pinned
             FROM collections ORDER BY pinned DESC, sort_order, created_at DESC",
        )?;
        let mut collections = statement
            .query_map([], |row| {
                Ok(SavedCollection {
                    id: row.get(0)?,
                    origin: enum_column(row.get::<_, String>(1)?),
                    kind: enum_column(row.get::<_, String>(2)?),
                    name: row.get(3)?,
                    owner: row.get(4)?,
                    artwork: row.get(5)?,
                    custom_artwork: row.get(6)?,
                    provider: row.get::<_, Option<String>>(7)?.map(enum_column),
                    source: row.get(8)?,
                    created_at: row.get(9)?,
                    pinned: row.get(10)?,
                    tracks: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for collection in &mut collections {
            collection.tracks = collection_tracks(&db, &collection.id)?;
        }

        Ok(Snapshot { liked, recent, collections })
    }

    pub fn set_liked(&self, track: &Track, liked: bool) -> Result<()> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        if liked {
            let key = upsert_track(&tx, track)?;
            let stamp = next_stamp(&tx, "SELECT MAX(added_at) FROM liked")?;
            tx.execute(
                "INSERT OR IGNORE INTO liked (track_key, added_at) VALUES (?1, ?2)",
                params![key, stamp],
            )?;
        } else {
            tx.execute("DELETE FROM liked WHERE track_key = ?1", params![track_key(track)])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn mark_played(&self, track: &Track) -> Result<()> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        let key = upsert_track(&tx, track)?;
        let stamp = next_stamp(&tx, "SELECT MAX(played_at) FROM recent")?;
        tx.execute(
            "INSERT INTO recent (track_key, played_at) VALUES (?1, ?2)
             ON CONFLICT(track_key) DO UPDATE SET played_at = excluded.played_at",
            params![key, stamp],
        )?;
        tx.execute(
            "DELETE FROM recent WHERE track_key NOT IN
             (SELECT track_key FROM recent ORDER BY played_at DESC LIMIT ?1)",
            params![RECENT_LIMIT],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Saves an imported collection; importing the same link again refreshes it in place.
    pub fn save_import(&self, collection: &Collection, source: &str) -> Result<SavedCollection> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        let existing: Option<(String, i64, Option<String>)> = tx
            .query_row(
                "SELECT id, created_at, custom_artwork FROM collections WHERE source = ?1",
                params![source],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (id, created_at, custom_artwork) =
            existing.unwrap_or_else(|| (new_id(), now_ms(), None));

        let top = top_sort_order(&tx)?;
        tx.execute(
            "INSERT INTO collections (id, origin, kind, name, owner, artwork, custom_artwork, provider, source, created_at, sort_order)
             VALUES (?1, 'import', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, name = excluded.name,
                 owner = excluded.owner, artwork = excluded.artwork, provider = excluded.provider",
            params![
                id,
                enum_text(&collection.kind),
                collection.name,
                collection.owner,
                collection.artwork,
                custom_artwork,
                enum_text(&collection.provider),
                source,
                created_at,
                top
            ],
        )?;
        replace_tracks(&tx, &id, &collection.tracks)?;
        tx.commit()?;
        drop(db);
        self.collection(&id)
    }

    pub fn create_playlist(&self, name: &str) -> Result<SavedCollection> {
        let id = new_id();
        let mut db = self.lock();
        let tx = db.transaction()?;
        let top = top_sort_order(&tx)?;
        tx.execute(
            "INSERT INTO collections (id, origin, kind, name, created_at, sort_order)
             VALUES (?1, 'user', 'playlist', ?2, ?3, ?4)",
            params![id, name, now_ms(), top],
        )?;
        tx.commit()?;
        drop(db);
        self.collection(&id)
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        let changed = self
            .lock()
            .execute("UPDATE collections SET pinned = ?2 WHERE id = ?1", params![id, pinned])?;
        expect_one(changed)
    }

    /// Stores the sidebar order chosen by dragging; ids not listed keep their place after.
    pub fn reorder_collections(&self, ids: &[String]) -> Result<()> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        for (order, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE collections SET sort_order = ?2 WHERE id = ?1",
                params![id, order as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        self.update_one("UPDATE collections SET name = ?2 WHERE id = ?1 AND origin = 'user'", id, name)
    }

    /// `None` restores the cover that came with the collection.
    pub fn set_cover(&self, id: &str, cover: Option<&str>) -> Result<()> {
        let changed = self
            .lock()
            .execute("UPDATE collections SET custom_artwork = ?2 WHERE id = ?1", params![id, cover])?;
        expect_one(changed)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let changed = self.lock().execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        expect_one(changed)
    }

    /// Appends tracks to a user playlist, skipping ones it already contains.
    pub fn add_tracks(&self, id: &str, tracks: &[Track]) -> Result<SavedCollection> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        ensure_user_playlist(&tx, id)?;
        let mut next: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM collection_tracks WHERE collection_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        for track in tracks {
            let key = upsert_track(&tx, track)?;
            let present: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM collection_tracks WHERE collection_id = ?1 AND track_key = ?2)",
                params![id, key],
                |row| row.get(0),
            )?;
            if !present {
                tx.execute(
                    "INSERT INTO collection_tracks (collection_id, position, track_key) VALUES (?1, ?2, ?3)",
                    params![id, next, key],
                )?;
                next += 1;
            }
        }
        tx.commit()?;
        drop(db);
        self.collection(id)
    }

    pub fn remove_track(&self, id: &str, track: &Track) -> Result<SavedCollection> {
        self.reorder_with(id, |tracks| tracks.retain(|key| key != &track_key(track)))
    }

    pub fn move_track(&self, id: &str, from: usize, to: usize) -> Result<SavedCollection> {
        self.reorder_with(id, |tracks| {
            if from < tracks.len() && to < tracks.len() {
                let key = tracks.remove(from);
                tracks.insert(to, key);
            }
        })
    }

    /// Imports the browser-storage library of older builds; safe to run more than once.
    pub fn import_legacy(&self, legacy: &LegacyLibrary) -> Result<()> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        let now = now_ms();
        for (offset, track) in legacy.liked.iter().enumerate() {
            let key = upsert_track(&tx, track)?;
            tx.execute(
                "INSERT OR IGNORE INTO liked (track_key, added_at) VALUES (?1, ?2)",
                params![key, now - offset as i64],
            )?;
        }
        for (offset, track) in legacy.recent.iter().enumerate() {
            let key = upsert_track(&tx, track)?;
            tx.execute(
                "INSERT OR IGNORE INTO recent (track_key, played_at) VALUES (?1, ?2)",
                params![key, now - offset as i64],
            )?;
        }
        for saved in &legacy.collections {
            let collection = &saved.collection;
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO collections (id, origin, kind, name, owner, artwork, provider, source, created_at)
                 VALUES (?1, 'import', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    saved.id,
                    enum_text(&collection.kind),
                    collection.name,
                    collection.owner,
                    collection.artwork,
                    enum_text(&collection.provider),
                    saved.source,
                    saved.imported_at
                ],
            )?;
            if inserted > 0 {
                replace_tracks(&tx, &saved.id, &collection.tracks)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn reorder_with(&self, id: &str, change: impl FnOnce(&mut Vec<String>)) -> Result<SavedCollection> {
        let mut db = self.lock();
        let tx = db.transaction()?;
        ensure_user_playlist(&tx, id)?;
        let mut keys: Vec<String> = tx
            .prepare("SELECT track_key FROM collection_tracks WHERE collection_id = ?1 ORDER BY position")?
            .query_map(params![id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        change(&mut keys);
        write_positions(&tx, id, &keys)?;
        tx.commit()?;
        drop(db);
        self.collection(id)
    }

    fn collection(&self, id: &str) -> Result<SavedCollection> {
        self.snapshot()?
            .collections
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| Error::Unavailable("playlist not found".into()))
    }

    fn update_one(&self, sql: &str, id: &str, value: &str) -> Result<()> {
        let changed = self.lock().execute(sql, params![id, value])?;
        expect_one(changed)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn migrate(db: &Connection) -> Result<()> {
    let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 1 {
        db.execute_batch(SCHEMA_V1)?;
    }
    db.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn tracks_by(db: &Connection, sql: &str) -> Result<Vec<Track>> {
    let rows: Vec<String> = db
        .prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    rows.iter().map(|data| Ok(serde_json::from_str(data)?)).collect()
}

fn collection_tracks(db: &Connection, id: &str) -> Result<Vec<Track>> {
    let rows: Vec<String> = db
        .prepare(
            "SELECT t.data FROM collection_tracks c JOIN tracks t ON t.key = c.track_key
             WHERE c.collection_id = ?1 ORDER BY c.position",
        )?
        .query_map(params![id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    rows.iter().map(|data| Ok(serde_json::from_str(data)?)).collect()
}

fn upsert_track(tx: &Transaction, track: &Track) -> Result<String> {
    let key = track_key(track);
    tx.execute(
        "INSERT INTO tracks (key, data) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET data = excluded.data",
        params![key, serde_json::to_string(track)?],
    )?;
    Ok(key)
}

fn replace_tracks(tx: &Transaction, id: &str, tracks: &[Track]) -> Result<()> {
    let keys = tracks
        .iter()
        .map(|track| upsert_track(tx, track))
        .collect::<Result<Vec<_>>>()?;
    write_positions(tx, id, &keys)
}

fn write_positions(tx: &Transaction, id: &str, keys: &[String]) -> Result<()> {
    tx.execute("DELETE FROM collection_tracks WHERE collection_id = ?1", params![id])?;
    let mut insert = tx.prepare(
        "INSERT INTO collection_tracks (collection_id, position, track_key) VALUES (?1, ?2, ?3)",
    )?;
    for (position, key) in keys.iter().enumerate() {
        insert.execute(params![id, position as i64, key])?;
    }
    Ok(())
}

fn ensure_user_playlist(tx: &Transaction, id: &str) -> Result<()> {
    let origin: Option<String> = tx
        .query_row("SELECT origin FROM collections WHERE id = ?1", params![id], |row| row.get(0))
        .optional()?;
    match origin.as_deref() {
        Some("user") => Ok(()),
        Some(_) => Err(Error::Unavailable("imported playlists are read-only".into())),
        None => Err(Error::Unavailable("playlist not found".into())),
    }
}

/// Current time, nudged past the newest stored stamp so rapid actions keep their order.
fn next_stamp(tx: &Transaction, newest_sql: &str) -> Result<i64> {
    let newest: Option<i64> = tx.query_row(newest_sql, [], |row| row.get(0))?;
    Ok(newest.map_or(now_ms(), |newest| now_ms().max(newest + 1)))
}

fn top_sort_order(tx: &Transaction) -> Result<i64> {
    Ok(tx.query_row("SELECT COALESCE(MIN(sort_order), 0) - 1 FROM collections", [], |row| row.get(0))?)
}

fn expect_one(changed: usize) -> Result<()> {
    if changed == 0 {
        return Err(Error::Unavailable("playlist not found".into()));
    }
    Ok(())
}

/// Same key the interface uses, so both sides agree on track identity.
fn track_key(track: &Track) -> String {
    format!("{}:{}", enum_text(&track.provider), track.id)
}

/// Serde's lowercase name of a unit enum variant, as stored in text columns.
fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn enum_column<T: for<'de> Deserialize<'de>>(text: String) -> T {
    serde_json::from_value(serde_json::Value::String(text))
        .expect("library database holds a value this build does not know")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("{:x}-{:x}", now_ms(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(n: u32) -> Track {
        Track {
            id: format!("id-{n}"),
            provider: Provider::Youtube,
            title: format!("Track {n}"),
            artists: vec!["Artist".into()],
            album: None,
            duration_ms: Some(180_000),
            artwork: None,
            url: None,
        }
    }

    fn titles(collection: &SavedCollection) -> Vec<String> {
        collection.tracks.iter().map(|t| t.title.clone()).collect()
    }

    #[test]
    fn likes_and_recent_history() {
        let library = Library::in_memory().unwrap();
        library.set_liked(&track(1), true).unwrap();
        library.set_liked(&track(2), true).unwrap();
        library.set_liked(&track(1), false).unwrap();
        for n in 0..30 {
            library.mark_played(&track(n)).unwrap();
        }
        library.mark_played(&track(3)).unwrap();

        let snapshot = library.snapshot().unwrap();
        assert_eq!(snapshot.liked.len(), 1);
        assert_eq!(snapshot.recent.len(), RECENT_LIMIT as usize);
        assert_eq!(snapshot.recent[0].title, "Track 3");
    }

    #[test]
    fn user_playlists_can_be_edited() {
        let library = Library::in_memory().unwrap();
        let playlist = library.create_playlist("Mine").unwrap();
        let id = playlist.id.as_str();

        let playlist = library.add_tracks(id, &[track(1), track(2), track(3), track(2)]).unwrap();
        assert_eq!(titles(&playlist), ["Track 1", "Track 2", "Track 3"]);

        let playlist = library.move_track(id, 2, 0).unwrap();
        assert_eq!(titles(&playlist), ["Track 3", "Track 1", "Track 2"]);

        let playlist = library.remove_track(id, &track(1)).unwrap();
        assert_eq!(titles(&playlist), ["Track 3", "Track 2"]);

        library.rename(id, "Renamed").unwrap();
        library.set_cover(id, Some("data:image/jpeg;base64,AAAA")).unwrap();
        let saved = library.collection(id).unwrap();
        assert_eq!(saved.name, "Renamed");
        assert_eq!(saved.custom_artwork.as_deref(), Some("data:image/jpeg;base64,AAAA"));

        library.delete(id).unwrap();
        assert!(library.snapshot().unwrap().collections.is_empty());
    }

    #[test]
    fn reimporting_a_link_refreshes_it_and_keeps_the_custom_cover() {
        let library = Library::in_memory().unwrap();
        let collection = Collection {
            kind: CollectionKind::Playlist,
            name: "Hits".into(),
            owner: None,
            artwork: Some("https://cover".into()),
            provider: Provider::Spotify,
            tracks: vec![track(1)],
        };
        let first = library.save_import(&collection, "link").unwrap();
        library.set_cover(&first.id, Some("data:custom")).unwrap();

        let updated = Collection { tracks: vec![track(1), track(2)], ..collection };
        let second = library.save_import(&updated, "link").unwrap();
        assert_eq!(second.id, first.id);
        assert_eq!(second.tracks.len(), 2);
        assert_eq!(second.custom_artwork.as_deref(), Some("data:custom"));
        assert!(library.add_tracks(&second.id, &[track(3)]).is_err(), "imports are read-only");
    }

    #[test]
    fn sidebar_order_and_pinning() {
        let library = Library::in_memory().unwrap();
        let a = library.create_playlist("A").unwrap().id;
        let b = library.create_playlist("B").unwrap().id;
        let c = library.create_playlist("C").unwrap().id;
        let names = |library: &Library| -> Vec<String> {
            library.snapshot().unwrap().collections.into_iter().map(|c| c.name).collect()
        };
        assert_eq!(names(&library), ["C", "B", "A"], "newest first");

        library.reorder_collections(&[a.clone(), c.clone(), b.clone()]).unwrap();
        assert_eq!(names(&library), ["A", "C", "B"]);

        library.set_pinned(&b, true).unwrap();
        assert_eq!(names(&library), ["B", "A", "C"], "pinned stay on top");
    }

    #[test]
    fn imports_legacy_browser_storage_once() {
        let library = Library::in_memory().unwrap();
        let legacy: LegacyLibrary = serde_json::from_value(serde_json::json!({
            "liked": [track(1)],
            "recent": [track(2), track(1)],
            "collections": [{
                "id": "abc", "source": "https://open.spotify.com/playlist/x", "importedAt": 5,
                "name": "Old", "owner": null, "artwork": null, "provider": "spotify",
                "tracks": [track(3)]
            }]
        }))
        .unwrap();
        library.import_legacy(&legacy).unwrap();
        library.import_legacy(&legacy).unwrap();

        let snapshot = library.snapshot().unwrap();
        assert_eq!(snapshot.liked.len(), 1);
        assert_eq!(snapshot.recent[0].title, "Track 2");
        assert_eq!(snapshot.collections.len(), 1);
        assert_eq!(snapshot.collections[0].origin, Origin::Import);
        assert_eq!(titles(&snapshot.collections[0]), ["Track 3"]);
    }
}

