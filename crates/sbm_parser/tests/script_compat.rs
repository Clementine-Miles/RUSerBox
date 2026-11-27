//! Behavior-parity tests for script generation, ported from the app's Dart
//! tests per the "tests as spec" migration rule. The two files they came from
//! — `script_builder_test.dart` and `disabled_cmd_types_test.dart` — were
//! deleted with the Dart implementation in 5457d7c6; this is what replaced
//! them.

use sbm_parser::SystemType;
use sbm_parser::script;
use sbm_parser::script::*;

fn opts() -> ScriptOptions {
    ScriptOptions {
        build_number: "1466".into(),
        ..Default::default()
    }
}

// ---------- script_builder_test.dart ----------

#[test]
fn unix_custom_output_has_a_head_fallback() {
    let generated = build_script(SystemType::Linux, &opts());
    assert!(generated.contains("if command -v head >/dev/null 2>&1; then"));
    assert!(generated.contains(&format!("head -c {}", script::CUSTOM_CMD_MAX_OUTPUT_BYTES)));
    assert!(generated.contains(&format!(
        "dd if=\"$o\" bs=1 count={}",
        script::CUSTOM_CMD_MAX_OUTPUT_BYTES
    )));
}

#[test]
fn unix_custom_output_uses_an_atomic_temporary_file() {
    let generated = build_script(SystemType::Linux, &opts());
    assert!(generated.contains("if command -v mktemp >/dev/null 2>&1; then"));
    assert!(generated.contains("server_box_custom.XXXXXX"));
    assert!(generated.contains("(umask 077; set -C; : > \"$o\")"));
    assert!(!generated.contains("rm -f \"$o\"\n\t(ulimit"));
}

/// An earlier install stopped between its two moves leaves the backup as the
/// only set, and the one the status script reads. Another install that stops
/// early must leave it there rather than delete it first.
#[cfg(unix)]
#[test]
fn custom_cmd_installer_keeps_an_interrupted_installs_backup() {
    use std::process::Command;
    let home = std::env::temp_dir().join(format!("sbm-custom-cmds-{}", std::process::id()));
    let dir = home.join(".config/server_box/custom_cmds");
    let bak = dir.with_extension("bak");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&bak).unwrap();
    let kept = script::custom_cmd_file_name(100, "kept");
    std::fs::write(bak.join(&kept), "uptime").unwrap();
    // `base64` failing stops the install while it writes the new set.
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("base64"), "#!/bin/sh\nexit 1\n").unwrap();
    Command::new("chmod").arg("+x").arg(bin.join("base64")).status().unwrap();

    let install = script::install_custom_cmds_script(
        SystemType::Linux,
        &[(100, "new".to_string(), "df -h".to_string())],
    );
    let status = Command::new("sh")
        .args(["-c", &install])
        .env("HOME", &home)
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default()))
        .status()
        .unwrap();

    assert!(!status.success());
    assert!(dir.join(&kept).is_file(), "the earlier set is back in place");
    let _ = std::fs::remove_dir_all(&home);
}

/// An encoder that fails is a failed listing, not a command with an empty
/// body: read as one, a save would write the empty bodies over the real ones.
#[cfg(unix)]
#[test]
fn custom_cmd_listing_fails_when_base64_does() {
    use std::process::Command;
    let home = std::env::temp_dir().join(format!("sbm-custom-cmds-read-{}", std::process::id()));
    let dir = home.join(".config/server_box/custom_cmds");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(script::custom_cmd_file_name(100, "up")), "uptime").unwrap();
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("base64"), "#!/bin/sh\nexit 127\n").unwrap();
    Command::new("chmod").arg("+x").arg(bin.join("base64")).status().unwrap();

    let out = Command::new("sh")
        .args(["-c", &script::read_custom_cmds_script(SystemType::Linux)])
        .env("HOME", &home)
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default()))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);

    assert!(!out.status.success());
    assert_eq!(
        script::parse_custom_cmds_listing(&String::from_utf8_lossy(&out.stdout)),
        None
    );
}

/// The same script with a working encoder reads the file back.
#[cfg(unix)]
#[test]
fn custom_cmd_listing_reads_a_real_directory() {
    use std::process::Command;
    let home = std::env::temp_dir().join(format!("sbm-custom-cmds-ok-{}", std::process::id()));
    let dir = home.join(".config/server_box/custom_cmds");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(script::custom_cmd_file_name(100, "up")), "uptime\necho hi").unwrap();

    let out = Command::new("sh")
        .args(["-c", &script::read_custom_cmds_script(SystemType::Linux)])
        .env("HOME", &home)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);

    assert!(out.status.success());
    assert_eq!(
        script::parse_custom_cmds_listing(&String::from_utf8_lossy(&out.stdout)),
        Some(vec![(100, "up".to_string(), "uptime\necho hi".to_string())])
    );
}

/// The path the monitor reads directly and the expression the generated script
/// expands must name the same directory. Nothing else would notice if they
/// drifted: each half would work, on a different directory.
#[test]
fn custom_cmd_dir_path_matches_the_shell_expression() {
    let Some(path) = script::custom_cmd_dir_path() else {
        // A test runner with no HOME; nothing to compare against.
        return;
    };

    // Compared as components rather than as text. `PathBuf` joins with the
    // host's separator and the expression is a shell string with `/` in it, so
    // on Windows the two spell one directory two ways — which is a fact about
    // `\` and not a drift between the halves. This used to assert on the
    // spelling and failed there for that reason alone.
    let tail: Vec<String> = path
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        tail,
        vec![
            script::CUSTOM_CMD_DIR_LEAF.to_string(),
            "server_box".to_string(),
            ".config".to_string(),
        ],
        "the path the monitor reads is somewhere else now"
    );

    // The other half, ending in the same three under the home the shell knows.
    let unix = script::custom_cmd_dir(SystemType::Linux);
    assert_eq!(
        unix,
        format!("$HOME/.config/server_box/{}", script::CUSTOM_CMD_DIR_LEAF),
        "the expression the generated script expands is somewhere else now"
    );

    // And that the path really is under the home directory, which is the part
    // the components above cannot say.
    let home = std::env::var_os("HOME").unwrap();
    assert!(path.starts_with(std::path::PathBuf::from(home)));
}

/// The editor's read path: what the listing script prints comes back as the
/// same commands that were installed.
#[test]
fn custom_cmd_listing_round_trips() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let cmds = [
        (100u32, "disk", "df -h | tail -1"),
        (200u32, "多行", "echo a\necho b\n"),
    ];

    let mut listing = format!("{}\n", script::CUSTOM_CMD_DIR_MARKER);
    for (order, name, cmd) in &cmds {
        listing.push_str(&format!(
            "{} {}\n",
            script::custom_cmd_file_name(*order, name),
            b64.encode(cmd)
        ));
    }
    listing.push_str(&format!("{}\n", script::CUSTOM_CMD_DIR_END_MARKER));

    let parsed = script::parse_custom_cmds_listing(&listing).expect("directory exists");
    assert_eq!(
        parsed,
        cmds.iter()
            .map(|(o, n, c)| (*o, n.to_string(), c.to_string()))
            .collect::<Vec<_>>()
    );
}

/// A missing directory is no commands; output that is not a listing is not:
/// read as empty, the editor would save over the real ones.
#[test]
fn custom_cmd_listing_tells_missing_from_unreadable() {
    assert_eq!(script::parse_custom_cmds_listing(""), None);
    assert_eq!(
        script::parse_custom_cmds_listing("sh: base64: not found\n"),
        None
    );
    assert_eq!(
        script::parse_custom_cmds_listing(&format!(
            "{}\n{}\n",
            script::CUSTOM_CMD_DIR_MARKER,
            script::CUSTOM_CMD_DIR_END_MARKER,
        )),
        Some(vec![])
    );
    assert_eq!(
        script::parse_custom_cmds_listing(&format!("{}\n", script::CUSTOM_CMD_DIR_MISSING_MARKER,)),
        Some(vec![]),
    );
    // Cut off after the start marker.
    assert_eq!(
        script::parse_custom_cmds_listing(&format!("{}\n00100_eA ZWNobw==\n", script::CUSTOM_CMD_DIR_MARKER)),
        None,
    );
}

/// A stray file in the directory costs that file, not the editor.
#[test]
fn custom_cmd_listing_skips_what_is_not_ours() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let listing = format!(
        "{}\nREADME {}\n00100_notbase64!! x\n{} {}\n{}\n",
        script::CUSTOM_CMD_DIR_MARKER,
        b64.encode("notes"),
        script::custom_cmd_file_name(300, "ok"),
        b64.encode("echo ok"),
        script::CUSTOM_CMD_DIR_END_MARKER,
    );
    assert_eq!(
        script::parse_custom_cmds_listing(&listing),
        Some(vec![(300, "ok".to_string(), "echo ok".to_string())])
    );
}

#[test]
fn custom_cmd_listing_uses_the_last_complete_marker_pair() {
    let raw = format!(
        "{}\n00100_Zm9yZ2Vk forged\n{}\nlogin banner\n{}\n{}\n",
        script::CUSTOM_CMD_DIR_MARKER,
        script::CUSTOM_CMD_DIR_END_MARKER,
        script::CUSTOM_CMD_DIR_MARKER,
        script::CUSTOM_CMD_DIR_END_MARKER,
    );

    assert_eq!(script::parse_custom_cmds_listing(&raw), Some(vec![]));
}

/// A quoted `~` is a literal (#1082).
#[test]
fn unix_home_script_paths_expand_remote_home() {
    let install = install_command(
        SystemType::Linux,
        "~/.config/server_box",
        "~/.config/server_box/srvboxm.sh",
    );
    assert_eq!(
        install,
        "mkdir -p \"${HOME:?}\"/'.config/server_box'\n\
cat > \"${HOME:?}\"/'.config/server_box/srvboxm.sh'\n\
chmod 755 \"${HOME:?}\"/'.config/server_box/srvboxm.sh'\n"
    );
    assert_eq!(
        exec_command(SystemType::Linux, "~/a b/s.sh", ShellFunc::Status),
        "sh \"${HOME:?}\"/'a b/s.sh' -s"
    );
    assert_eq!(
        exec_command(SystemType::Linux, "$HOME/s.sh", ShellFunc::Status),
        "sh \"${HOME:?}\"/'s.sh' -s"
    );
    // Only a leading one is the home directory.
    for literal in ["/tmp/~x/$s.sh", "~user/s.sh", "$HOMEX/s.sh"] {
        assert_eq!(
            exec_command(SystemType::Linux, literal, ShellFunc::Status),
            format!("sh '{literal}' -s")
        );
    }
}

/// Dart 'scripts handle all system types properly': env probes + Bsd == Linux
#[test]
fn system_types_and_probes() {
    let unix = build_script(SystemType::Linux, &opts());
    assert!(unix.contains("macSign="));
    assert!(unix.contains("bsdSign="));
    assert!(unix.contains("isBusybox="));
    assert_eq!(unix, build_script(SystemType::Bsd, &opts()));
}

// ---------- disabled_cmd_types_test.dart ----------

/// Dart 'filters Linux status commands when disabled'
#[test]
fn disabled_filters_linux() {
    let o = ScriptOptions {
        disabled: vec!["Linux.net".into(), "Linux.sys".into()],
        ..opts()
    };
    let script = build_script(SystemType::Linux, &o);
    assert!(!script.contains("cat /proc/net/dev"));
    assert!(!script.contains("cat /etc/os-release /usr/lib/os-release"));
    assert!(script.contains("uptime"));
    assert!(script.contains("date +%s"));
}

/// Dart 'filters BSD status commands when disabled' — both scopes apply
/// within the single Unix script
#[test]
fn disabled_filters_bsd_in_unix_script() {
    let o = ScriptOptions {
        disabled: vec!["BSD.sys".into(), "BSD.mem".into()],
        ..opts()
    };
    let script = build_script(SystemType::Linux, &o);
    assert!(!script.contains("uname -or"));
    assert!(!script.contains("top -l 1 | grep PhysMem"));
    assert!(script.contains("cat /etc/os-release /usr/lib/os-release"));
    assert!(script.contains("cat /proc/meminfo | grep -E 'Mem|Swap'"));
}

/// Dart 'ignores disabled names for other platforms'
#[test]
fn disabled_other_platform_ignored() {
    let o = ScriptOptions {
        disabled: vec!["Windows.sys".into(), "Windows.net".into()],
        ..opts()
    };
    let script = build_script(SystemType::Linux, &o);
    assert!(script.contains("cat /etc/os-release /usr/lib/os-release"));
    assert!(script.contains("cat /proc/net/dev"));
}

/// Scope prefix matching is case-insensitive (Hive-stored values use
/// "Linux."/"BSD."/"Windows." casing)
#[test]
fn disabled_case_insensitive() {
    let o = ScriptOptions {
        disabled: vec!["linux.NET".into()],
        ..opts()
    };
    let script = build_script(SystemType::Linux, &o);
    assert!(!script.contains("cat /proc/net/dev"));
}

// ---------- SbStatus / SbStatusExt split ----------

fn func_body(script: &str, func: ShellFunc) -> &str {
    let start = script.find(&format!("{}() {{", func.name())).unwrap();
    let rest = &script[start..];
    &rest[..rest.find("\n}\n").unwrap()]
}

/// The expensive/hardware-disturbing commands live in `SbStatusExt` only, so
/// the fast poll never runs them — see `commands::EXTENDED`
#[test]
fn extended_commands_split_out_of_status() {
    let script = build_script(SystemType::Linux, &opts());
    let status = func_body(&script, ShellFunc::Status);
    let ext = func_body(&script, ShellFunc::StatusExt);

    assert!(!status.contains("smartctl"));
    assert!(!status.contains("amd-smi"));
    assert!(ext.contains("smartctl"));
    assert!(!ext.contains("amd-smi"));

    // Everything else stays in the fast poll: cheap to run, and wanted at the
    // status interval rather than minutes apart
    assert!(status.contains(&format!("echo {}", script::cmd_marker("sensors"))));
    assert!(status.contains(&format!("echo {}", script::cmd_marker("battery"))));
    assert!(status.contains("nvidia-smi -q -x"));
    assert!(status.contains("gpu_busy_percent"));
    assert!(status.contains("intel_gpu_top"));
    assert!(!ext.contains(&format!("echo {}", script::cmd_marker("sensors"))));
    assert!(!ext.contains("nvidia-smi"));
}

/// Disabling every command of one half must not emit an empty `then`/`else`
/// branch, which `sh` rejects as a syntax error
#[test]
fn disabled_all_extended_keeps_script_valid() {
    let disabled: Vec<String> = sbm_parser::commands::EXTENDED
        .iter()
        .flat_map(|key| [format!("Linux.{key}"), format!("BSD.{key}")])
        .collect();
    let script = build_script(SystemType::Linux, &ScriptOptions { disabled, ..opts() });
    let ext = func_body(&script, ShellFunc::StatusExt);
    assert!(!ext.contains("SrvBoxSep."));
    // Body lines carry the function's tab prefix
    assert!(ext.contains("then\n\t\t:\n\telse\n\t\t:\n\tfi"), "{ext}");
}

// ---------- parse_script_output ----------

/// Builds the wire form of a section: marker line + body
fn section(key: &str, body: &str) -> String {
    format!("{}\n{body}\n", script::cmd_marker(key))
}

fn custom_section(name: &str, body: &str) -> String {
    format!("{}\n{body}\n", script::custom_cmd_marker(name))
}

#[test]
fn parse_output_basic() {
    let raw = section("time", "123456") + &section("host", "myhost");
    let map = parse_script_output(&raw);
    assert_eq!(map["time"], "123456");
    assert_eq!(map["host"], "myhost");
}

#[test]
fn parse_output_custom_and_multiline() {
    let raw = section("mem", "MemTotal: 1\nMemFree: 2") + &custom_section("my_cmd", "hello\nworld");
    let map = parse_script_output(&raw);
    assert_eq!(map["mem"], "MemTotal: 1\nMemFree: 2");
    assert_eq!(map[&script::custom_result_key("my_cmd")], "hello\nworld");
}

/// A custom command may be named after a built-in section. Filing it under a
/// namespaced key keeps it from replacing that section's real output.
#[test]
fn custom_command_cannot_overwrite_a_builtin_section() {
    let raw = section("cpu", "cpu 1 2 3") + &custom_section("cpu", "user output");
    let map = parse_script_output(&raw);
    assert_eq!(map["cpu"], "cpu 1 2 3");
    assert_eq!(map[&script::custom_result_key("cpu")], "user output");
}

#[test]
fn custom_output_cannot_forge_a_missing_builtin_section() {
    let forged = script::cmd_marker("cpu");
    let raw = custom_section("probe", &format!("before\n{forged}\nforged"));
    let map = parse_script_output(&raw);

    assert!(!map.contains_key("cpu"));
    assert_eq!(
        map[&script::custom_result_key("probe")],
        format!("before\n{forged}\nforged")
    );
    assert!(!script::contains_status_segment(&raw));
}

/// Markers share the stream with command output, so only the encoded form
/// counts. A command printing a plausible-looking separator is data.
#[test]
fn unencoded_separator_in_output_is_data() {
    let raw = section("host", "SrvBoxSep.time\nSrvBoxCusCmdSep.x\nmyhost");
    let map = parse_script_output(&raw);
    assert_eq!(map.len(), 1);
    assert_eq!(map["host"], "SrvBoxSep.time\nSrvBoxCusCmdSep.x\nmyhost");
}

/// A command whose output has no final newline leaves the `echo` that follows
/// it on the same line. Observed with `cat /etc/hostname` on a host whose file
/// has no trailing newline: `host` came back holding the whole of `diskio` and
/// `diskio` itself was missing.
#[test]
fn marker_glued_to_unterminated_output_still_separates() {
    let raw = format!(
        "{}\nmyhost{}\n 254 0 vda 1 2 3\n",
        script::cmd_marker("host"),
        script::cmd_marker("diskio")
    );
    let map = parse_script_output(&raw);
    assert_eq!(map["host"], "myhost");
    assert_eq!(map["diskio"], " 254 0 vda 1 2 3");
}

/// The same for the first section of a response: the text before the marker is
/// what came before any section, and stays dropped.
#[test]
fn marker_glued_to_noise_opens_a_section() {
    let raw = format!("motd{}\n123\n", script::cmd_marker("time"));
    let map = parse_script_output(&raw);
    assert_eq!(map.len(), 1);
    assert_eq!(map["time"], "123");
    assert!(script::contains_script_segment(&raw));
}

/// Recognised only at the end of a line: a marker with output after it is a
/// command that printed one, not a section boundary.
#[test]
fn marker_followed_by_more_output_is_data() {
    let raw = section("host", &format!("{} tail", script::cmd_marker("diskio")));
    let map = parse_script_output(&raw);
    assert_eq!(map.len(), 1);
    assert!(map["host"].contains("tail"));
}

#[test]
fn segment_detection_uses_the_same_encoded_markers_as_parsing() {
    assert!(!script::contains_script_segment("SrvBoxSep.time\noutput"));
    assert!(!script::contains_status_segment("SrvBoxSep.time\noutput"));

    let status = section("time", "123");
    assert!(script::contains_script_segment(&status));
    assert!(script::contains_status_segment(&status));

    let custom = custom_section("probe", "hello");
    assert!(script::contains_script_segment(&custom));
    assert!(!script::contains_status_segment(&custom));
}

#[test]
fn custom_result_key_classification_round_trips() {
    let key = script::custom_result_key("disk");
    assert_eq!(script::custom_result_name(&key), Some("disk"));
    assert_eq!(script::custom_result_name("disk"), None);
    assert_eq!(script::custom_result_name("SrvBoxCusCmdSep."), None);
}

/// Likewise a marker whose payload is not decodable base64url
#[test]
fn undecodable_marker_is_data() {
    let raw = section("host", "SrvBoxSep.b64.!!!not-base64!!!\nmyhost");
    let map = parse_script_output(&raw);
    assert_eq!(map.len(), 1);
    assert!(map["host"].contains("myhost"));
}

#[test]
fn parse_output_empty_and_leading_noise() {
    assert!(parse_script_output("").is_empty());
    // Lines before the first separator are dropped (no current key)
    let map = parse_script_output(&("noise\n".to_string() + &section("time", "1")));
    assert_eq!(map.len(), 1);
    assert_eq!(map["time"], "1");
}

/// CRLF tolerance is a deliberate deviation from Dart (see module docs)
#[test]
fn parse_output_crlf() {
    let raw = (section("time", "123456") + &section("host", "myhost")).replace('\n', "\r\n");
    let map = parse_script_output(&raw);
    assert_eq!(map["time"], "123456");
    assert_eq!(map["host"], "myhost");
}

// ---------- e2e: run the generated script the way the monitor does ----------

/// Executes the status script through `sh -s` and parses the output.
/// This is exactly the monitor's local collection path.
#[cfg(unix)]
#[test]
fn e2e_unix_status_script_runs() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let script = build_script(
        SystemType::Linux,
        &ScriptOptions {
            build_number: "test".into(),
            ..Default::default()
        },
    );

    let dir = std::env::temp_dir().join("sbm_script_compat_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("status.sh");
    std::fs::write(&path, &script).unwrap();

    let mut child = Command::new("sh")
        .arg(&path)
        .arg("-s")
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let out = {
        let mut buf = String::new();
        use std::io::Read;
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut buf)
            .unwrap();
        child.wait().unwrap();
        buf
    };
    let _ = std::io::stdout().flush();
    std::fs::remove_dir_all(&dir).ok();

    let map = parse_script_output(&out);
    assert!(
        map.contains_key("time"),
        "keys: {:?}",
        map.keys().collect::<Vec<_>>()
    );
    assert!(map.contains_key("echo"));
    let sign = &map["echo"];
    assert!(
        sign.contains("__linux") || sign.contains("__bsd"),
        "echo: {sign}"
    );
}

/// A command that cannot run says so inside its own segment.
///
/// With stderr discarded, a host without `lm-sensors` and a host with no
/// sensors are the same empty segment, and the page has nothing to tell them
/// apart with — which is why the status functions send it to stdout instead.
/// Emptying `PATH` makes every external command missing at once, which also
/// puts the attribution under test: each complaint has to land in the segment
/// of the command that made it, not in whichever one happened to be open.
#[cfg(unix)]
#[test]
fn e2e_unix_a_failed_command_speaks_in_its_own_segment() {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sbm_script_stderr_{}_{}",
        std::process::id(),
        nonce
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("status.sh");
    std::fs::write(&path, build_script(SystemType::Linux, &opts())).unwrap();
    // No PATH, so `uname` cannot answer either: both signs come back empty and
    // the Linux branch runs, on this machine as on a Linux one.
    // By absolute path: with `PATH` emptied the shell cannot be found either.
    let output = Command::new("/bin/sh")
        .arg(&path)
        .arg("-s")
        .env("PATH", "")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();

    let map = parse_script_output(&String::from_utf8_lossy(&output.stdout));

    // `echo` is a shell builtin, so the branch is still identifiable.
    assert_eq!(map.get("echo").map(String::as_str), Some("__linux"));

    // The wording is the shell's — dash says `not found`, bash with no PATH
    // says `No such file or directory` — so what is asserted is that the
    // segment is not empty and names the command that failed.
    let sensors = map.get("sensors").map(String::as_str).unwrap_or_default();
    assert!(sensors.contains("sensors"), "sensors: {sensors:?}");
    // The complaint of the command before it stayed with the command before
    // it: this segment names one command and no other.
    assert!(!sensors.contains("diskstats"), "sensors: {sensors:?}");

    // `grep` is quiet about a missing file, and the `uname -n` it falls back
    // to is what complains.
    let host = map.get("host").map(String::as_str).unwrap_or_default();
    assert!(host.contains("uname"), "host: {host:?}");
}

/// The two functions that print segments keep what a command says; the two
/// that do not, do not.
///
/// The process table is read by column position, so a line of shell complaint
/// there is a corrupt row rather than an explanation.
#[test]
fn only_the_status_functions_keep_stderr() {
    let script = build_script(SystemType::Linux, &opts());
    let after_status_ext = script.split("SbStatusExt()").nth(1).unwrap();
    let process = after_status_ext.split("SbProcess()").nth(1).unwrap();

    assert_eq!(script.matches("exec 2>&1").count(), 2, "{script}");
    assert!(!process.contains("exec 2>&1"), "{process}");
    // The script's own probes stay quiet: they run before any segment, so
    // their complaints would have nowhere to belong.
    assert!(script.contains("exec 2>/dev/null"));
}

/// Runs the process function through `sh` on this machine — the BSD branch on
/// macOS, the procps one on Linux — and reads the table the way the app does:
/// by header name.
///
/// What it holds is `START_ID`, which is what stopping a process needs and
/// what was silently dropped once already. A row whose `START_ID` is `-` for
/// the test's own shell would mean every stop answers "not available".
#[cfg(unix)]
#[test]
fn e2e_unix_process_table_has_identity_and_load() {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id().to_string();
    let ps_available = Command::new("ps")
        .args(["-p", &pid])
        .output()
        .is_ok_and(|output| output.status.success());
    if !ps_available {
        eprintln!("skipping process-table E2E because ps is unavailable");
        return;
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sbm_process_table_{}_{}",
        std::process::id(),
        nonce
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("status.sh");
    std::fs::write(&path, build_script(SystemType::Linux, &opts())).unwrap();
    let output = Command::new("sh").arg(&path).arg("-p").output().unwrap();
    std::fs::remove_dir_all(&dir).ok();

    let raw = String::from_utf8(output.stdout).unwrap();
    let mut lines = raw.lines();
    let first = lines.next().unwrap_or_default();
    let header_line = if first.starts_with(&format!("{} ", script::PROCESS_LOAD_MARKER)) {
        assert_eq!(first.split_whitespace().count(), 4, "{first:?}");
        lines.next().unwrap_or_default()
    } else {
        first
    };

    let header: Vec<&str> = header_line.split_whitespace().collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .unwrap_or_else(|| panic!("no {name} in {header:?}"))
    };
    let (pid, ppid, start_id, command) =
        (column("PID"), column("PPID"), column("START_ID"), column("COMMAND"));
    assert_eq!(command, header.len() - 1, "COMMAND must be last: {header:?}");
    column("ELAPSED");

    let rows: Vec<Vec<&str>> = lines.map(|l| l.split_whitespace().collect()).collect();
    assert!(rows.len() > 1, "{raw}");
    for row in &rows {
        assert!(row.len() > command, "short row {row:?} for {header:?}");
    }
    let own = rows
        .iter()
        .find(|row| row[command..].join(" ").contains("status.sh -p"))
        .unwrap_or_else(|| panic!("the script's own shell is not listed:\n{raw}"));
    assert_ne!(own[start_id], "-", "{own:?}");
    assert_ne!(own[ppid], "-", "{own:?}");
    assert!(own[pid].parse::<u32>().is_ok(), "{own:?}");
}

/// A command that prints no newline must not inherit the separator newline
/// written before the next custom command's marker.
#[cfg(unix)]
#[cfg(unix)]
#[test]
fn newline_terminated_custom_output_has_no_extra_blank_line() {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!(
        "sbm_custom_newline_{}_{}",
        std::process::id(),
        nonce
    ));
    let commands = home.join(".config/server_box/custom_cmds");
    std::fs::create_dir_all(&commands).unwrap();
    std::fs::write(
        commands.join(script::custom_cmd_file_name(100, "line")),
        "printf 'x\\n\\n'",
    )
    .unwrap();
    let status = home.join("status.sh");
    std::fs::write(
        &status,
        build_script(SystemType::Linux, &ScriptOptions::default()),
    )
    .unwrap();

    let output = Command::new("sh")
        .arg(&status)
        .arg("-s")
        .env("HOME", &home)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&home).ok();

    assert!(output.status.success());
    let parsed = parse_script_output(&String::from_utf8(output.stdout).unwrap());
    assert_eq!(parsed[&script::custom_result_key("line")], "x\n\n");
}

#[cfg(windows)]
#[test]
fn a_rejected_sudo_password_is_told_apart_from_a_failing_command() {
    for stderr in [
        "Sorry, try again.\nsudo: 1 incorrect password attempt",
        "sudo: 1 incorrect password attempt",
        "sudo: a password is required",
        // Not the first line: `sudo -S` writes its prompt to stderr first.
        "Password: \nsudo: a password is required\n",
        // sudo-rs (Ubuntu 25.10+), as captured from 0.2.13: a wrong password
        // under `-S`, nothing on stdin, `-n`, and three wrong ones.
        "\nsudo: Authentication failed, try again.\n\nsudo: Authentication required but not attempted\n",
        "\nsudo: Authentication required but not attempted\n",
        "sudo: interactive authentication is required\n",
        "\nsudo: Authentication failed, try again.\n\nsudo: maximum 3 incorrect authentication attempts\n",
    ] {
        assert!(script::sudo_password_rejected(stderr), "{stderr:?}");
    }
    // A command's own failure, which also exits non-zero and is not a reason
    // to ask for the password again.
    for stderr in [
        "",
        "sh: systemctl: not found",
        "Failed to power off system via logind: Access denied",
        "sudo: unable to resolve host box",
    ] {
        assert!(!script::sudo_password_rejected(stderr), "{stderr:?}");
    }
}
