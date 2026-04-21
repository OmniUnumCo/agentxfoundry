use std::path::PathBuf;

#[test]
fn daemon_binary_resolved_as_sibling() {
    let exe = PathBuf::from("/usr/local/bin/rr-mcp");
    let result = rr::daemon::resolve_daemon_binary(&exe);
    assert_eq!(result, PathBuf::from("/usr/local/bin/rr-daemon"));
}

#[test]
fn daemon_binary_resolved_when_already_daemon() {
    let exe = PathBuf::from("/usr/local/bin/rr-daemon");
    let result = rr::daemon::resolve_daemon_binary(&exe);
    assert_eq!(result, PathBuf::from("/usr/local/bin/rr-daemon"));
}

#[test]
fn daemon_binary_resolved_from_bare_path() {
    let exe = std::path::PathBuf::from("rr-mcp");
    let result = rr::daemon::resolve_daemon_binary(&exe);
    assert_eq!(result, std::path::PathBuf::from("rr-daemon"));
}
