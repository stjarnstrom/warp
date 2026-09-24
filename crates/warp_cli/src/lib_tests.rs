use super::*;

#[test]
fn identifies_worker_subcommands() {
    assert!(is_worker_invocation("minidump-server"));
    #[cfg(unix)]
    assert!(is_worker_invocation(&terminal_server_subcommand()));
    assert!(!is_worker_invocation("--prompt"));
    assert!(is_worker_invocation("remote-server-proxy"));
    assert!(is_worker_invocation("remote-server-daemon"));
    assert!(is_worker_invocation("ripgrep-search"));
    assert!(!is_worker_invocation("completions"));
    assert!(!is_worker_invocation("agent"));
}

#[test]
fn desktop_urls_and_settings_schema_still_parse() {
    let args = Args::try_parse_from(["warp", "warp://launch"]).unwrap();
    assert_eq!(args.app_args().urls[0].as_str(), "warp://launch");
    let args = Args::try_parse_from(["warp", "dump-settings-schema"]).unwrap();
    assert!(matches!(
        args.command(),
        Some(Command::DumpSettingsSchema { output_path: None })
    ));
}

#[test]
fn removed_agent_commands_are_rejected() {
    assert!(Args::try_parse_from(["warp", "agent", "run", "--prompt", "hello"]).is_err());
    assert!(Args::try_parse_from(["warp", "login"]).is_err());
    let help = Args::clap_command().render_long_help().to_string();
    assert!(!help.contains("run-cloud"));
    assert!(!help.contains("cloud agents"));
}

#[test]
fn hidden_server_overrides_still_parse() {
    let args = Args::try_parse_from([
        "warp",
        "--server-root-url",
        "http://localhost:8080",
        "--ws-server-url",
        "ws://localhost:8082/graphql/v2",
        "--session-sharing-server-url",
        "http://localhost:8083",
    ])
    .unwrap();
    assert_eq!(args.server_root_url(), Some("http://localhost:8080"));
    assert_eq!(args.ws_server_url(), Some("ws://localhost:8082/graphql/v2"));
    assert_eq!(
        args.session_sharing_server_url(),
        Some("http://localhost:8083")
    );
}
