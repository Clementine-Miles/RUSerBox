use sbm_parser::zellij;

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_find_command() -> String {
    zellij::FIND_COMMAND.to_owned()
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_parse_find(stdout: String) -> Option<String> {
    zellij::parse_find(&stdout)
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_list_sessions_command(bin: String) -> String {
    zellij::list_sessions_command(&bin)
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_read_sessions(stdout: String, stderr: String, exit_code: i32) -> Option<Vec<String>> {
    zellij::read_sessions(&stdout, &stderr, exit_code)
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_valid_name(name: String) -> bool {
    zellij::valid_name(&name)
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_attach_command(bin: String, name: String, create: bool) -> Option<String> {
    zellij::attach_command(&bin, &name, create)
}

#[flutter_rust_bridge::frb(sync)]
pub fn zellij_kill_session_command(bin: String, name: String) -> Option<String> {
    zellij::kill_session_command(&bin, &name)
}
