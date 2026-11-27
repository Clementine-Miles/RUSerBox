//! Where the store's 32-byte key lives between launches.
//!
//! On Android the key is sealed by an AES-GCM key held in the Android
//! Keystore (`KeyVault.java`) and only the sealed bytes are written to
//! `store.key`; the raw key never touches disk. The desktop development
//! build keeps it in a 0600 file beside the store, and says so.
//!
//! A store that exists is never given a new key: a missing or unreadable key
//! is an error the interface reports, so the file can still be opened if the
//! key turns up.

use std::path::{Path, PathBuf};

pub const KEY_FILE: &str = "store.key";

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("the store exists but its key is gone; nothing was reset")]
    Lost,
    #[error("the key could not be read: {0}")]
    Unreadable(String),
    #[error("the key could not be saved: {0}")]
    Write(String),
}

/// Seals and opens the key (Android: the Keystore; desktop: identity).
pub trait Sealer {
    fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, String>;
    fn open(&self, sealed: &[u8]) -> Result<[u8; 32], String>;
    /// A key the previous (Flutter) build kept, migrated once.
    fn legacy(&self) -> Option<[u8; 32]> {
        None
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(KEY_FILE)
}

/// The key for the store in [dir]: read, migrated from the old vault, or —
/// only when there is no store yet — created.
pub fn load(dir: &Path, sealer: &dyn Sealer) -> Result<[u8; 32], KeyError> {
    let file = path(dir);
    if let Ok(sealed) = std::fs::read(&file) {
        return sealer.open(&sealed).map_err(KeyError::Unreadable);
    }
    let key = match sealer.legacy() {
        Some(k) => k,
        None if sbm_store::Store::exists(dir) => return Err(KeyError::Lost),
        None => sbm_store::new_key(),
    };
    let sealed = sealer.seal(&key).map_err(KeyError::Write)?;
    write_private(&file, &sealed).map_err(|e| KeyError::Write(e.to_string()))?;
    // Read back: a write that did not land would lose the store for good.
    let back = std::fs::read(&file).map_err(|e| KeyError::Write(e.to_string()))?;
    let opened = sealer.open(&back).map_err(KeyError::Write)?;
    if opened != key {
        return Err(KeyError::Write("the saved key does not read back".into()));
    }
    Ok(key)
}

fn write_private(file: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = file.with_extension("key.tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().create(true).truncate(true).write(true).mode(0o600).open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    #[cfg(not(unix))]
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, file)
}

/// The development build's "sealer": none. Never used on Android.
pub struct Plain;

impl Sealer for Plain {
    fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
        Ok(key.to_vec())
    }
    fn open(&self, sealed: &[u8]) -> Result<[u8; 32], String> {
        sealed.try_into().map_err(|_| "the key file is damaged".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Xor;
    impl Sealer for Xor {
        fn seal(&self, k: &[u8; 32]) -> Result<Vec<u8>, String> {
            Ok(k.iter().map(|b| b ^ 0x5a).collect())
        }
        fn open(&self, s: &[u8]) -> Result<[u8; 32], String> {
            let v: Vec<u8> = s.iter().map(|b| b ^ 0x5a).collect();
            v.try_into().map_err(|_| "bad".to_owned())
        }
    }

    #[test]
    fn a_key_is_made_once_and_never_for_an_existing_store() {
        let dir = tempfile::tempdir().unwrap();
        let k1 = load(dir.path(), &Xor).unwrap();
        assert_eq!(load(dir.path(), &Xor).unwrap(), k1);
        assert_ne!(std::fs::read(path(dir.path())).unwrap(), k1.to_vec(), "only sealed bytes on disk");
        // A store without its key file is refused, not reset.
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join(sbm_store::FILE_NAME), b"x").unwrap();
        assert!(matches!(load(other.path(), &Xor), Err(KeyError::Lost)));
    }
}
