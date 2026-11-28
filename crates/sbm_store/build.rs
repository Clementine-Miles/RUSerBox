//! Compiles the SQLite3 Multiple Ciphers amalgamation: the same library the
//! Flutter app linked (`sqlite3` with `source: sqlite3mc`), so an existing
//! `store.db` opens with its key unchanged.

fn main() {
    let dir = sqlite3mc_src::source_dir();
    let mut build = cc::Build::new();
    build
        .file(dir.join(sqlite3mc_src::SOURCE_FILE))
        .include(dir)
        .warnings(false)
        .define("SQLITE_THREADSAFE", "1")
        .define("SQLITE_DQS", "0")
        .define("SQLITE_DEFAULT_MEMSTATUS", "0")
        .define("SQLITE_OMIT_LOAD_EXTENSION", None)
        .define("SQLITE_TEMP_STORE", "2")
        .define("SQLITE_ENABLE_FTS5", None)
        // The cipher the Flutter build named for new files; existing files
        // record theirs in the header.
        .define("CODEC_TYPE", "CODEC_TYPE_CHACHA20");
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android") {
        // Bionic has no separate libpthread/libdl.
        build.define("HAVE_USLEEP", "1");
    }
    build.compile("sqlite3mc");
    println!("cargo:rerun-if-changed=build.rs");
}
