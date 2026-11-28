//! A small safe wrapper over the SQLite C API, linked against the
//! SQLite3 Multiple Ciphers build from `build.rs`.
//!
//! Only what the store uses: open, key, execute, and row-by-row queries with
//! bound parameters. Every statement is prepared, bound and finalized inside
//! one call, so no statement handle outlives the borrow of its connection.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

use crate::Error;

#[allow(non_camel_case_types)]
type sqlite3 = c_void;
#[allow(non_camel_case_types)]
type sqlite3_stmt = c_void;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_NOTADB: c_int = 26;
const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
const SQLITE_OPEN_FULLMUTEX: c_int = 0x0001_0000;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_TEXT: c_int = 3;
const SQLITE_BLOB: c_int = 4;
const SQLITE_NULL: c_int = 5;
/// `SQLITE_TRANSIENT`: SQLite copies the bound bytes before the call returns.
const SQLITE_TRANSIENT: isize = -1;

unsafe extern "C" {
    fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut sqlite3, flags: c_int, vfs: *const c_char) -> c_int;
    fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_extended_errcode(db: *mut sqlite3) -> c_int;
    fn sqlite3_prepare_v2(db: *mut sqlite3, sql: *const c_char, n: c_int, stmt: *mut *mut sqlite3_stmt, tail: *mut *const c_char) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_bind_null(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, i: c_int, v: i64) -> c_int;
    fn sqlite3_bind_double(stmt: *mut sqlite3_stmt, i: c_int, v: f64) -> c_int;
    fn sqlite3_bind_text(stmt: *mut sqlite3_stmt, i: c_int, v: *const c_char, n: c_int, d: isize) -> c_int;
    fn sqlite3_bind_blob(stmt: *mut sqlite3_stmt, i: c_int, v: *const c_void, n: c_int, d: isize) -> c_int;
    fn sqlite3_column_count(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_type(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, i: c_int) -> i64;
    fn sqlite3_column_double(stmt: *mut sqlite3_stmt, i: c_int) -> f64;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, i: c_int) -> *const u8;
    fn sqlite3_column_blob(stmt: *mut sqlite3_stmt, i: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_changes(db: *mut sqlite3) -> c_int;
}

/// A value bound to or read from a statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_owned())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}
impl From<&String> for Value {
    fn from(v: &String) -> Self {
        Value::Text(v.clone())
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}
impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Int(v.into())
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Int(v.into())
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Value::Null, Into::into)
    }
}

/// One result row, by column index.
pub struct Row(Vec<Value>);

impl Row {
    pub fn get(&self, i: usize) -> &Value {
        self.0.get(i).unwrap_or(&Value::Null)
    }
    pub fn text(&self, i: usize) -> Option<String> {
        match self.get(i) {
            Value::Text(s) => Some(s.clone()),
            Value::Int(n) => Some(n.to_string()),
            Value::Real(f) => Some(f.to_string()),
            _ => None,
        }
    }
    pub fn text_or_default(&self, i: usize) -> String {
        self.text(i).unwrap_or_default()
    }
    pub fn int(&self, i: usize) -> Option<i64> {
        match self.get(i) {
            Value::Int(n) => Some(*n),
            Value::Real(f) => Some(*f as i64),
            Value::Text(s) => s.parse().ok(),
            _ => None,
        }
    }
    pub fn real(&self, i: usize) -> Option<f64> {
        match self.get(i) {
            Value::Real(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }
    pub fn bool(&self, i: usize) -> bool {
        self.int(i).unwrap_or(0) != 0
    }
}

/// An open connection. Not `Sync`: the store serializes access behind a
/// mutex, and the library is built `SQLITE_THREADSAFE=1` besides.
pub struct Connection {
    db: *mut sqlite3,
}

// SAFETY: the handle is opened FULLMUTEX and only used through `&mut`/`&`
// from one thread at a time (the store wraps it in a `Mutex`).
unsafe impl Send for Connection {}

impl Connection {
    /// Opens (creating) the file at [path].
    pub fn open(path: &Path) -> Result<Self, Error> {
        let c = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| Error::Path)?;
        Self::open_c(&c)
    }

    /// An unencrypted in-memory database, for tests.
    pub fn open_in_memory() -> Result<Self, Error> {
        Self::open_c(c":memory:")
    }

    fn open_c(name: &CStr) -> Result<Self, Error> {
        let mut db = ptr::null_mut();
        // SAFETY: valid NUL-terminated name and out-pointer.
        let rc = unsafe { sqlite3_open_v2(name.as_ptr(), &mut db, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX, ptr::null()) };
        let conn = Connection { db };
        if rc != SQLITE_OK {
            return Err(conn.error(rc));
        }
        Ok(conn)
    }

    /// Applies the cipher and a raw 32-byte key, then reads a page so a wrong
    /// key fails here as [`Error::KeyRejected`] rather than later as an empty
    /// store.
    pub fn key(&self, cipher: &str, key: &[u8]) -> Result<(), Error> {
        self.execute_batch(&format!("PRAGMA cipher = '{cipher}';"))?;
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        let mut pragma = zeroize::Zeroizing::new(format!("PRAGMA key = \"x'{hex}'\";"));
        let res = self.execute_batch(&pragma);
        zeroize::Zeroize::zeroize(&mut *pragma);
        res?;
        match self.query("SELECT count(*) FROM sqlite_master", &[], |_| Ok(())) {
            Ok(_) => Ok(()),
            Err(Error::Sqlite { code, .. }) if code & 0xff == SQLITE_NOTADB => Err(Error::KeyRejected),
            Err(e) => Err(e),
        }
    }

    fn error(&self, rc: c_int) -> Error {
        if self.db.is_null() {
            return Error::Sqlite { code: rc, message: "out of memory".into() };
        }
        // SAFETY: valid handle; the message is copied before any other call.
        let message = unsafe { CStr::from_ptr(sqlite3_errmsg(self.db)) }.to_string_lossy().into_owned();
        let code = unsafe { sqlite3_extended_errcode(self.db) };
        Error::Sqlite { code: if code != 0 { code } else { rc }, message }
    }

    /// Runs one or more statements with no parameters and no rows.
    pub fn execute_batch(&self, sql: &str) -> Result<(), Error> {
        let mut rest = sql.trim();
        while !rest.is_empty() {
            let c = CString::new(rest).map_err(|_| Error::Sql("NUL in SQL".into()))?;
            let mut stmt = ptr::null_mut();
            let mut tail: *const c_char = ptr::null();
            // SAFETY: valid handle and buffers; `tail` points into `c`.
            let rc = unsafe { sqlite3_prepare_v2(self.db, c.as_ptr(), -1, &mut stmt, &mut tail) };
            if rc != SQLITE_OK {
                return Err(self.error(rc));
            }
            let consumed = if tail.is_null() { rest.len() } else { tail as usize - c.as_ptr() as usize };
            if !stmt.is_null() {
                let rc = loop {
                    // SAFETY: a freshly prepared statement.
                    let rc = unsafe { sqlite3_step(stmt) };
                    if rc != SQLITE_ROW {
                        break rc;
                    }
                };
                unsafe { sqlite3_finalize(stmt) };
                if rc != SQLITE_DONE {
                    return Err(self.error(rc));
                }
            }
            rest = rest[consumed..].trim_start();
        }
        Ok(())
    }

    /// Runs one statement with [params]; answers the rows it changed.
    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, Error> {
        self.run(sql, params, &mut |_| Ok(()))?;
        // SAFETY: valid handle.
        Ok(unsafe { sqlite3_changes(self.db) } as usize)
    }

    /// Runs one statement with [params], mapping each row through [f].
    pub fn query<T>(&self, sql: &str, params: &[Value], mut f: impl FnMut(&Row) -> Result<T, Error>) -> Result<Vec<T>, Error> {
        let mut out = Vec::new();
        self.run(sql, params, &mut |row| {
            out.push(f(row)?);
            Ok(())
        })?;
        Ok(out)
    }

    /// The first row through [f], if any.
    pub fn query_one<T>(&self, sql: &str, params: &[Value], f: impl FnMut(&Row) -> Result<T, Error>) -> Result<Option<T>, Error> {
        Ok(self.query(sql, params, f)?.into_iter().next())
    }

    fn run(&self, sql: &str, params: &[Value], each: &mut dyn FnMut(&Row) -> Result<(), Error>) -> Result<(), Error> {
        let c = CString::new(sql).map_err(|_| Error::Sql("NUL in SQL".into()))?;
        let mut stmt = ptr::null_mut();
        // SAFETY: valid handle and NUL-terminated SQL.
        let rc = unsafe { sqlite3_prepare_v2(self.db, c.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(self.error(rc));
        }
        if stmt.is_null() {
            return Ok(());
        }
        let guard = Stmt(stmt);
        for (i, p) in params.iter().enumerate() {
            let idx = (i + 1) as c_int;
            // SAFETY: `stmt` is live; TRANSIENT makes SQLite copy the bytes.
            let rc = unsafe {
                match p {
                    Value::Null => sqlite3_bind_null(stmt, idx),
                    Value::Int(n) => sqlite3_bind_int64(stmt, idx, *n),
                    Value::Real(f) => sqlite3_bind_double(stmt, idx, *f),
                    Value::Text(s) => sqlite3_bind_text(stmt, idx, s.as_ptr().cast(), s.len() as c_int, SQLITE_TRANSIENT),
                    Value::Blob(b) => sqlite3_bind_blob(stmt, idx, b.as_ptr().cast(), b.len() as c_int, SQLITE_TRANSIENT),
                }
            };
            if rc != SQLITE_OK {
                return Err(self.error(rc));
            }
        }
        loop {
            // SAFETY: `stmt` is live until `guard` drops.
            let rc = unsafe { sqlite3_step(stmt) };
            match rc {
                SQLITE_ROW => {
                    let n = unsafe { sqlite3_column_count(stmt) };
                    let mut row = Vec::with_capacity(n as usize);
                    for i in 0..n {
                        row.push(unsafe { column(stmt, i) });
                    }
                    each(&Row(row))?;
                }
                SQLITE_DONE => break,
                rc => return Err(self.error(rc)),
            }
        }
        drop(guard);
        Ok(())
    }

    /// Runs [f] inside `BEGIN IMMEDIATE … COMMIT`, rolling back on error.
    pub fn transaction<T>(&self, f: impl FnOnce(&Connection) -> Result<T, Error>) -> Result<T, Error> {
        self.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(v) => {
                self.execute_batch("COMMIT")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}

/// Reads column [i] of the current row.
///
/// # Safety
/// `stmt` must be a live statement positioned on a row.
unsafe fn column(stmt: *mut sqlite3_stmt, i: c_int) -> Value {
    unsafe {
        match sqlite3_column_type(stmt, i) {
            SQLITE_INTEGER => Value::Int(sqlite3_column_int64(stmt, i)),
            SQLITE_FLOAT => Value::Real(sqlite3_column_double(stmt, i)),
            SQLITE_TEXT => {
                let p = sqlite3_column_text(stmt, i);
                let n = sqlite3_column_bytes(stmt, i) as usize;
                if p.is_null() {
                    Value::Text(String::new())
                } else {
                    Value::Text(String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned())
                }
            }
            SQLITE_BLOB => {
                let p = sqlite3_column_blob(stmt, i);
                let n = sqlite3_column_bytes(stmt, i) as usize;
                if p.is_null() { Value::Blob(Vec::new()) } else { Value::Blob(std::slice::from_raw_parts(p.cast::<u8>(), n).to_vec()) }
            }
            SQLITE_NULL => Value::Null,
            _ => Value::Null,
        }
    }
}

struct Stmt(*mut sqlite3_stmt);

impl Drop for Stmt {
    fn drop(&mut self) {
        // SAFETY: finalized exactly once.
        unsafe { sqlite3_finalize(self.0) };
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if !self.db.is_null() {
            // SAFETY: every statement is finalized before its call returns.
            unsafe { sqlite3_close_v2(self.db) };
        }
    }
}
