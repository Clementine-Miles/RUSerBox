//! Zellij CLI commands. Discovery and termination use SSH exec. Attach runs
//! in a PTY, so Zellij renders the session and handles its own keybindings.

use crate::script::shell_quote_unix as quote;

pub const FIND_COMMAND: &str =
    "command -v zellij 2>/dev/null || bash -i -c 'command -v zellij' 2>/dev/null";

pub fn parse_find(stdout: &str) -> Option<String> {
    // The interactive Bash fallback can print a startup banner before the path.
    stdout
        .lines()
        .rfind(|line| line.starts_with('/') && line.ends_with("/zellij"))
        .map(str::to_owned)
}

pub fn list_sessions_command(bin: &str) -> String {
    format!("{} list-sessions --no-formatting", quote(bin))
}

/// Returns live session names only. Zellij includes resurrectable sessions in
/// its listing, even with `--short`, so parse the suffix in the plain listing.
/// Exit 1 also covers real errors and means empty only with the no-session
/// diagnostic.
pub fn read_sessions(stdout: &str, stderr: &str, exit_code: i32) -> Option<Vec<String>> {
    const NO_SESSIONS: &str = "No active zellij sessions found.";
    let lines = stdout.lines().filter(|line| !line.trim().is_empty());
    if exit_code != 0 {
        return (exit_code == 1
            && ((stdout.trim().is_empty() && stderr.trim() == NO_SESSIONS)
                || (stderr.trim().is_empty() && stdout.trim() == NO_SESSIONS)))
            .then(Vec::new);
    }
    lines
        .map(|line| {
            let (name, description) = line.rsplit_once(" [Created ")?;
            let (_, suffix) = description.split_once(" ago]")?;
            match suffix.trim() {
                "" | "(current)" => Some(valid_name(name).then(|| name.to_owned())),
                "(EXITED - attach to resurrect)" => Some(None),
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(|listed| listed.into_iter().flatten().collect())
}

pub fn valid_name(name: &str) -> bool {
    // The CLI interprets a leading dash as an option. A slash can be a path
    // separator or a remote URL, which the SSH-local session manager never
    // needs. `help` and `options` would be parsed as attach subcommands.
    // Controls would interfere with the line-based listing.
    !name.trim().is_empty()
        && !matches!(name, "." | ".." | "help" | "options")
        && !name.starts_with('-')
        && !name.contains('/')
        && !name.chars().any(char::is_control)
}

pub fn attach_command(bin: &str, name: &str, create: bool) -> Option<String> {
    // Override a remote config selecting `quit` on forced close. Dropping an
    // SSH PTY disconnects this client without ending the session for others.
    let create_flag = if create { " --create" } else { "" };
    valid_name(name).then(|| {
        format!(
            "exec {} attach{create_flag} {} options --on-force-close detach",
            quote(bin),
            quote(name)
        )
    })
}

pub fn kill_session_command(bin: &str, name: &str) -> Option<String> {
    valid_name(name).then(|| format!("{} kill-session {}", quote(bin), quote(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_uses_only_the_binary_path() {
        assert_eq!(
            parse_find("Welcome back\n/home/test/.cargo/bin/zellij\n"),
            Some("/home/test/.cargo/bin/zellij".into())
        );
        assert_eq!(parse_find("zellij\n"), None);
    }

    #[test]
    fn reads_live_sessions_and_ignores_exited_sessions() {
        assert_eq!(
            read_sessions(
                "work [Created 2hours ago] (current)\n\
                 old [Created 1h ago] (EXITED - attach to resurrect)\n\
                 a [Created label ago] b [Created 3s ago] \n",
                "",
                0
            ),
            Some(vec!["work".into(), "a [Created label ago] b".into()])
        );
        assert_eq!(
            read_sessions("", "No active zellij sessions found.\n", 1),
            Some(vec![])
        );
        assert_eq!(read_sessions("", "Permission denied", 1), None);
        assert_eq!(read_sessions("unexpected response", "", 0), None);
    }

    #[test]
    fn commands_quote_full_names_and_reject_invalid_targets() {
        let bin = "/path with spaces/zellij";
        assert_eq!(
            attach_command(bin, "a'b", true).as_deref(),
            Some(
                r"exec '/path with spaces/zellij' attach --create 'a'\''b' options --on-force-close detach"
            )
        );
        assert_eq!(
            attach_command(bin, "main", false).as_deref(),
            Some("exec '/path with spaces/zellij' attach 'main' options --on-force-close detach")
        );
        assert_eq!(
            kill_session_command(bin, "main").as_deref(),
            Some("'/path with spaces/zellij' kill-session 'main'")
        );
        for name in [
            "",
            "  ",
            "..",
            "../other",
            "--help",
            "abc\nxyz",
            "http://example.org",
        ] {
            assert!(attach_command(bin, name, false).is_none());
            assert!(kill_session_command(bin, name).is_none());
        }
    }
}
