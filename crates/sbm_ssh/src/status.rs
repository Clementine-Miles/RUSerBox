//! Server status over SSH: detect the system, install the status script
//! (`sbm_parser::script`), run it, and parse what it prints.
//!
//! The script's name carries [`SCRIPT_VERSION`]: a server keeps the copy it
//! has until the version changes. It is written to `/tmp/server_box`, and to
//! `~/.config/server_box` when `/tmp` refuses it.

use std::collections::HashMap;
use std::time::Duration;

use sbm_parser::SystemType;
use sbm_parser::script::{self, ScriptOptions, ShellFunc};

use crate::{Error, Result, Session};

/// The number in the remote script's file name. It only increases, and does
/// whenever the generated script changes (the Flutter app's
/// `ScriptConstants.version`, continued).
pub const SCRIPT_VERSION: u32 = 91;

/// Where the script is written on a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPaths {
    pub dir: String,
    pub path: String,
}

impl ScriptPaths {
    /// The default directory for [system], or [custom] when the server has one.
    pub fn new(system: SystemType, custom: Option<&str>, home: bool) -> Self {
        let win = system == SystemType::Windows;
        let dir = match custom.map(str::trim).filter(|d| !d.is_empty()) {
            Some(d) => d.trim_end_matches(['/', '\\']).to_owned(),
            None => match (win, home) {
                (false, false) => "/tmp/server_box".into(),
                (false, true) => "~/.config/server_box".into(),
                (true, false) => "$env:TEMP/server_box".into(),
                (true, true) => "$env:USERPROFILE/.config/server_box".into(),
            },
        };
        let (sep, file) = if win { ('\\', format!("srvboxm_v{SCRIPT_VERSION}.ps1")) } else { ('/', format!("srvboxm_v{SCRIPT_VERSION}.sh")) };
        ScriptPaths { path: format!("{dir}{sep}{file}"), dir }
    }
}

/// Runs the status script on one session. Keeps which directory worked.
pub struct StatusRunner {
    pub system: SystemType,
    pub custom_dir: Option<String>,
    pub disabled: Vec<String>,
    installed: Option<ScriptPaths>,
    pub timeout: Duration,
}

impl StatusRunner {
    pub fn new(system: SystemType, custom_dir: Option<String>, disabled: Vec<String>) -> Self {
        StatusRunner { system, custom_dir, disabled, installed: None, timeout: Duration::from_secs(20) }
    }

    /// Asks the server which system it runs.
    pub async fn detect(session: &Session) -> Result<SystemType> {
        let t = Duration::from_secs(10);
        let out = session.exec(sbm_parser::system::UNAME_PROBE, None, t).await?;
        if let Some(s) = sbm_parser::system::from_uname(&out.stdout_str()) {
            return Ok(s);
        }
        let out = session.exec(sbm_parser::system::VER_PROBE, None, t).await?;
        Ok(sbm_parser::system::from_ver(&out.stdout_str()).unwrap_or(SystemType::Linux))
    }

    async fn install(&mut self, session: &Session) -> Result<ScriptPaths> {
        if let Some(p) = &self.installed {
            return Ok(p.clone());
        }
        let opts = ScriptOptions { disabled: self.disabled.clone(), build_number: SCRIPT_VERSION.to_string() };
        let content = script::install_payload(self.system, &script::build_script(self.system, &opts));
        let mut last = String::new();
        for home in [false, true] {
            if home && self.custom_dir.is_some() {
                break;
            }
            let paths = ScriptPaths::new(self.system, self.custom_dir.as_deref(), home);
            let cmd = script::install_command(self.system, &paths.dir, &paths.path);
            let out = session.exec(&cmd, Some(content.as_bytes()), self.timeout).await?;
            if out.ok() {
                self.installed = Some(paths.clone());
                return Ok(paths);
            }
            last = out.stderr_str();
        }
        Err(Error::Other(format!("the status script could not be written: {}", last.trim())))
    }

    /// Runs one shell function of the script, installing it first if needed.
    pub async fn run(&mut self, session: &Session, func: ShellFunc) -> Result<String> {
        let paths = self.install(session).await?;
        let out = session.exec(&script::exec_command(self.system, &paths.path, func), None, self.timeout).await?;
        let raw = out.stdout_str();
        if func == ShellFunc::Status && !script::contains_script_segment(&raw) {
            // A rebooted machine took /tmp with it: write it again next time.
            self.installed = None;
            return Err(Error::Other(format!("the status script printed nothing usable: {}", out.stderr_str().trim())));
        }
        Ok(raw)
    }

    /// One status reading, parsed.
    pub async fn status(&mut self, session: &Session, temp_divisor: f64) -> Result<sbm_parser::ServerStatus> {
        let raw = self.run(session, ShellFunc::Status).await?;
        Ok(parse(self.system, &raw, temp_divisor))
    }
}

/// Parses raw script output into a status.
pub fn parse(system: SystemType, raw: &str, temp_divisor: f64) -> sbm_parser::ServerStatus {
    let map: HashMap<String, String> = script::parse_script_output(raw);
    sbm_parser::parse_status_opts(system, &map, sbm_parser::ParseOptions { temp_divisor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_paths_follow_the_platform() {
        let p = ScriptPaths::new(SystemType::Linux, None, false);
        assert_eq!(p.dir, "/tmp/server_box");
        assert_eq!(p.path, format!("/tmp/server_box/srvboxm_v{SCRIPT_VERSION}.sh"));
        let p = ScriptPaths::new(SystemType::Bsd, Some("/opt/sb/"), true);
        assert_eq!(p.path, format!("/opt/sb/srvboxm_v{SCRIPT_VERSION}.sh"));
        let p = ScriptPaths::new(SystemType::Windows, None, true);
        assert!(p.path.ends_with(".ps1") && p.path.contains('\\'));
    }
}
