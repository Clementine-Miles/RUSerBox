//! What a remote host must not be able to do to the process parsing its output.
//!
//! Every input here is text a server sent back. In the app the parser runs
//! in-process over FFI, so a panic or an unbounded allocation is not an error a
//! caller can catch — it is the app disappearing. None of these need a hostile
//! server either: each is also a shape a broken tool or a corrupt reading
//! produces.

use sbm_parser::{bsd, linux};

/// The BSD twin: `sysctl -n hw.ncpu` arrives as a bare integer and is
/// replicated into that many pseudo-cores.
#[test]
fn an_absurd_bsd_core_count_falls_back_to_one_core() {
    let cores = bsd::parse_cpu("CPU usage: 14.70% user, 12.76% sys, 72.52% idle\n99999999999\n");
    assert_eq!(cores.len(), 1);
    assert_eq!(cores[0].id, "cpu0");
}

/// A plausible count is still honoured.
#[test]
fn a_real_bsd_core_count_is_still_honoured() {
    let cores = bsd::parse_cpu("CPU usage: 14.70% user, 12.76% sys, 72.52% idle\n10\n");
    assert_eq!(cores.len(), 10);
}

/// `df`'s size column was split one *byte* from the end to read its unit, which
/// lands inside a multi-byte character and panics.
#[test]
fn a_multi_byte_character_in_a_df_size_is_not_a_panic() {
    let raw = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/sda1 10G° 5 5 50% /\n";
    let disks = linux::parse_disk(raw);
    assert!(disks.is_empty(), "an unreadable size is not a disk");
}

/// The degenerate version: the whole column is one multi-byte character.
#[test]
fn a_df_size_that_is_only_a_multi_byte_character_is_not_a_panic() {
    let raw = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/sda1 ° 5 5 50% /\n";
    assert!(linux::parse_disk(raw).is_empty());
}

/// `df -h` suffixes still convert.
#[test]
fn a_df_h_suffix_still_converts() {
    let raw = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/sda1 10G 5G 5G 50% /\n";
    let disks = linux::parse_disk(raw);
    assert_eq!(disks.len(), 1);
    assert_eq!(disks[0].size, 10 * 1024 * 1024);
}
