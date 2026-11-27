//! `kv(store, key, value, updated_at)`: JSON values grouped by store, as
//! fl_lib's `SqliteStore` wrote them.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{Result, Store, Value, now_ms};

/// The key-value stores this build reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvStore {
    /// Preferences.
    Setting,
    /// A server's saved sudo password, by server id.
    SudoPassword,
    /// A private key's saved passphrase, by key id.
    KeyPassphrase,
    /// Remembered paths and inputs.
    History,
}

impl KvStore {
    pub fn name(self) -> &'static str {
        match self {
            KvStore::Setting => "setting",
            KvStore::SudoPassword => "sudoPassword",
            KvStore::KeyPassphrase => "keyPassphrase",
            KvStore::History => "history",
        }
    }
}

/// Internal keys carry this prefix (`StoreDefaults.prefixKey`).
pub const PREFIX: &str = "__lkpt_";

pub struct Kv<'a> {
    pub(crate) store: &'a Store,
    pub(crate) name: &'static str,
}

impl Kv<'_> {
    /// The raw JSON text under [key].
    pub fn raw(&self, key: &str) -> Result<Option<String>> {
        self.store.conn().query_one("SELECT value FROM kv WHERE store = ? AND key = ?", &[self.name.into(), key.into()], |r| Ok(r.text_or_default(0)))
    }

    /// The value under [key], or `None` when absent or not of type `T`.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.raw(key).ok().flatten().and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn get_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    pub fn set<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.store.conn().execute(
            "INSERT INTO kv (store, key, value, updated_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT (store, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            &[self.name.into(), key.into(), json.into(), Value::Int(now_ms())],
        )?;
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        self.store.conn().execute("DELETE FROM kv WHERE store = ? AND key = ?", &[self.name.into(), key.into()])?;
        Ok(())
    }

    pub fn keys(&self) -> Result<Vec<String>> {
        self.store.conn().query("SELECT key FROM kv WHERE store = ? ORDER BY key", &[self.name.into()], |r| Ok(r.text_or_default(0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_as_json() {
        let s = Store::in_memory().unwrap();
        let kv = s.kv(KvStore::Setting);
        kv.set("timeOut", &7).unwrap();
        kv.set("themeMode", &2).unwrap();
        assert_eq!(kv.get::<i64>("timeOut"), Some(7));
        assert_eq!(kv.raw("themeMode").unwrap().as_deref(), Some("2"));
        assert_eq!(kv.get::<String>("timeOut"), None, "a value of another type reads as absent");
        kv.remove("timeOut").unwrap();
        assert_eq!(kv.get::<i64>("timeOut"), None);
        assert!(s.kv(KvStore::SudoPassword).keys().unwrap().is_empty(), "stores are apart");
    }
}
