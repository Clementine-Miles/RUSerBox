//! The app's storage: one SQLite file (`store.db`) encrypted with
//! SQLite3 Multiple Ciphers (ChaCha20-Poly1305, a raw 32-byte key held by the
//! platform keystore), in the shape the last Flutter release left it.
//!
//! - Tables are the Drift schema at v43, frozen in `schema_v43.sql`; steps
//!   after it are [`schema::MIGRATIONS`]. A file below v43 was written by an
//!   older Flutter build whose own migrations (Hive import, id remapping) were
//!   never ported: it is refused with [`Error::SchemaTooOld`], never reset.
//! - Relational records (`server`, `private_key`, `snippet`, `port_forward`)
//!   own tables; preferences are JSON values in `kv(store, key, …)`.
//! - Ids are keys, never names; enums are stored by name; a delete writes a
//!   `tombstone` and an edit bumps `rev`, so a file stays readable by the sync
//!   format it was written for.

mod kv;
pub mod model;
pub mod schema;
mod sqlite;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub use kv::{Kv, KvStore};
pub use model::*;
pub use sqlite::{Connection, Row, Value};

/// The file name, beside where the Flutter app kept it.
pub const FILE_NAME: &str = "store.db";

/// The cipher new files are written with. Existing files record theirs.
pub const CIPHER: &str = "chacha20";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQLite error {code}: {message}")]
    Sqlite { code: i32, message: String },
    #[error("the key does not open this database (another key, or a damaged file)")]
    KeyRejected,
    #[error("the database path is not valid")]
    Path,
    #[error("invalid SQL: {0}")]
    Sql(String),
    #[error("schema v{stored} was written by an older build; open it once in the last Flutter release (v43) first")]
    SchemaTooOld { stored: i64 },
    #[error("schema v{stored} was written by a newer build (this one reads up to v{supported})")]
    SchemaTooNew { stored: i64, supported: i64 },
    #[error("a record named {0:?} already exists")]
    DuplicateName(String),
    #[error("{0}")]
    Invalid(String),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The open store. Cloneable handles are `Arc<Store>`.
pub struct Store {
    conn: Mutex<Connection>,
    path: Option<PathBuf>,
}

impl Store {
    /// Opens (creating) `dir/store.db` with [key], then migrates it.
    pub fn open(dir: &Path, key: &[u8; 32]) -> Result<Self> {
        let path = dir.join(FILE_NAME);
        let conn = Connection::open(&path)?;
        conn.key(CIPHER, key)?;
        Self::init(conn, Some(path))
    }

    /// Whether a non-empty store exists in [dir]: a key must then already
    /// exist, and a missing one is never replaced by a new one.
    pub fn exists(dir: &Path) -> bool {
        std::fs::metadata(dir.join(FILE_NAME)).is_ok_and(|m| m.len() > 0)
    }

    /// An unencrypted in-memory store, for tests.
    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Self> {
        if path.is_some() {
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        }
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::migrate(&conn)?;
        Ok(Store { conn: Mutex::new(conn), path })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The connection, for the typed accessors in [`model`].
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A key-value store (`setting`, `sudoPassword`, …).
    pub fn kv(&self, store: KvStore) -> Kv<'_> {
        Kv { store: self, name: store.name() }
    }
}

/// 32 bytes from the OS CSPRNG: a new store's key.
pub fn new_key() -> [u8; 32] {
    let mut k = [0u8; 32];
    getrandom::fill(&mut k).expect("the OS random source is unavailable");
    k
}

/// Milliseconds since the epoch, as `updated_at` holds them.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// A new record id: 16 random bytes, hex. The Flutter app wrote UUID v4
/// strings; both are opaque text keys.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_encrypted_and_opens_only_with_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let key = new_key();
        {
            let st = Store::open(dir.path(), &key).unwrap();
            st.put_server(&Server::new_ssh("secret-host", "10.1.2.3", 22, "root")).unwrap();
        }
        let raw = std::fs::read(dir.path().join(FILE_NAME)).unwrap();
        assert!(!raw.starts_with(b"SQLite format 3"), "no plaintext header");
        assert!(!raw.windows(11).any(|w| w == b"secret-host"));
        assert!(matches!(Store::open(dir.path(), &new_key()), Err(Error::KeyRejected)));
        let st = Store::open(dir.path(), &key).unwrap();
        assert_eq!(st.servers().unwrap()[0].name, "secret-host");
        assert!(Store::exists(dir.path()));
    }
}
