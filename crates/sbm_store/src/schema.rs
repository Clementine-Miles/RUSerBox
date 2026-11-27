//! Schema versions after the Flutter release's v43.
//!
//! The version lives where that release kept it, `kv('setting',
//! '__lkpt_schemaVersion')`, so a file moves between the two builds' numbering
//! without a second counter. A step is appended to [`MIGRATIONS`], never
//! edited, and keeps a test fed by a file in the shape before it.

use crate::kv::PREFIX;
use crate::sqlite::{Connection, Value};
use crate::{Error, Result, now_ms};

/// What the frozen DDL creates.
pub const BASE: i64 = 43;

/// v44: the removed AI Agent's stores and preferences, and the removed local
/// terminal's preferences, deleted. Their rows held provider credentials,
/// prompts and conversations that nothing reads any more; leaving them in an
/// encrypted file nobody can see is no protection worth keeping.
const M044: &str = "
DELETE FROM kv WHERE store IN ('llm', 'tool', 'memory', 'llmCredential', 'mcpSecret', 'chats', 'skills', 'agent');
DELETE FROM kv WHERE store = 'setting' AND (
    key LIKE 'agent%' OR key LIKE 'ai%' OR key LIKE 'llm%' OR key LIKE 'mcp%'
    OR key LIKE 'localTerminal%' OR key LIKE 'rootfs%' OR key LIKE 'proot%'
);
DELETE FROM sync_record WHERE c IN ('llm', 'tool', 'memory', 'llmCredential', 'mcpSecret', 'skill', 'skills');
DELETE FROM sync_passed WHERE c IN ('llm', 'tool', 'memory', 'llmCredential', 'mcpSecret', 'skill', 'skills');
";

/// Steps in order: `MIGRATIONS[i]` takes `BASE + i` to `BASE + i + 1`.
pub const MIGRATIONS: &[&str] = &[M044];

/// The version this build writes.
pub const CURRENT: i64 = BASE + MIGRATIONS.len() as i64;

const VERSION_KEY: &str = "schemaVersion";

fn version_key() -> String {
    format!("{PREFIX}{VERSION_KEY}")
}

/// The stored version, `None` for a file nothing has written yet.
pub fn stored(conn: &Connection) -> Result<Option<i64>> {
    let has_kv = conn.query_one("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'kv'", &[], |_| Ok(()))?.is_some();
    if !has_kv {
        return Ok(None);
    }
    let raw = conn.query_one("SELECT value FROM kv WHERE store = 'setting' AND key = ?", &[version_key().into()], |r| Ok(r.text_or_default(0)))?;
    Ok(raw.and_then(|s| s.trim().parse().ok()))
}

fn has_tables(conn: &Connection) -> Result<bool> {
    Ok(conn
        .query_one("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'server'", &[], |_| Ok(()))?
        .is_some())
}

fn write_version(conn: &Connection, v: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO kv (store, key, value, updated_at) VALUES ('setting', ?, ?, ?) \
         ON CONFLICT (store, key) DO UPDATE SET value = excluded.value",
        &[version_key().into(), v.to_string().into(), Value::Int(now_ms())],
    )?;
    Ok(())
}

/// Creates a fresh file's tables, or brings an existing one up to
/// [`CURRENT`]. Refuses (never resets) a file it cannot read.
pub fn migrate(conn: &Connection) -> Result<()> {
    let version = match stored(conn)? {
        Some(v) => v,
        None if has_tables(conn)? => {
            // Tables but no version: a Flutter build died between creating
            // them and recording it. Its own migrations may not have run.
            return Err(Error::SchemaTooOld { stored: 0 });
        }
        None => {
            conn.transaction(|c| {
                c.execute_batch(include_str!("schema_v43.sql"))?;
                write_version(c, BASE)
            })?;
            BASE
        }
    };
    if version < BASE {
        return Err(Error::SchemaTooOld { stored: version });
    }
    if version > CURRENT {
        return Err(Error::SchemaTooNew { stored: version, supported: CURRENT });
    }
    // Indexes are `IF NOT EXISTS`; re-applying the frozen DDL repairs a file
    // that lost one without touching data.
    conn.execute_batch(include_str!("schema_v43.sql"))?;
    for (i, step) in MIGRATIONS.iter().enumerate().skip((version - BASE) as usize) {
        conn.transaction(|c| {
            c.execute_batch(step)?;
            write_version(c, BASE + i as i64 + 1)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_file_is_created_at_current() {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        assert_eq!(stored(&c).unwrap(), Some(CURRENT));
        migrate(&c).unwrap();
        assert_eq!(stored(&c).unwrap(), Some(CURRENT), "idempotent");
    }

    /// A v43 file as the Flutter release left it, with the Agent's rows.
    #[test]
    fn v44_drops_the_agent_and_keeps_the_rest() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("schema_v43.sql")).unwrap();
        c.execute_batch(
            "INSERT INTO kv VALUES ('setting', '__lkpt_schemaVersion', '43', 0);
             INSERT INTO kv VALUES ('setting', 'timeOut', '9', 0);
             INSERT INTO kv VALUES ('setting', 'agentRedact', 'true', 0);
             INSERT INTO kv VALUES ('llmCredential', 'openai', '\"sk-secret\"', 0);
             INSERT INTO kv VALUES ('chats', 'c1', '{}', 0);
             INSERT INTO kv VALUES ('sudoPassword', 's1', '\"pw\"', 0);
             INSERT INTO server (id, name, auto_connect, ssh_ip, ssh_port, ssh_user, ssh_allow_legacy_algorithms, ssh_enabled, monitor_enabled, is_local, temp_is_celsius, disk_counts_network, ssh_forward_agent)
               VALUES ('s1', 'box', 1, '10.0.0.1', 22, 'root', 0, 1, 0, 0, 1, 0, 0);",
        )
        .unwrap();
        migrate(&c).unwrap();
        assert_eq!(stored(&c).unwrap(), Some(44));
        let left: Vec<(String, String)> =
            c.query("SELECT store, key FROM kv WHERE key != '__lkpt_schemaVersion' ORDER BY store, key", &[], |r| Ok((r.text_or_default(0), r.text_or_default(1)))).unwrap();
        assert_eq!(left, [("setting".into(), "timeOut".into()), ("sudoPassword".into(), "s1".into())]);
        let servers: Vec<String> = c.query("SELECT id FROM server", &[], |r| Ok(r.text_or_default(0))).unwrap();
        assert_eq!(servers, ["s1"]);
    }

    #[test]
    fn older_and_newer_files_are_refused_untouched() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("schema_v43.sql")).unwrap();
        c.execute_batch("INSERT INTO kv VALUES ('setting', '__lkpt_schemaVersion', '30', 0);").unwrap();
        assert!(matches!(migrate(&c), Err(Error::SchemaTooOld { stored: 30 })));
        c.execute_batch("UPDATE kv SET value = '999' WHERE key = '__lkpt_schemaVersion';").unwrap();
        assert!(matches!(migrate(&c), Err(Error::SchemaTooNew { stored: 999, .. })));
        assert_eq!(stored(&c).unwrap(), Some(999));
    }
}
