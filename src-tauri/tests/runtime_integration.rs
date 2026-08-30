use std::fs;

use tempfile::TempDir;
use token_tracing_widget_lib::collection::FixedClock;
use token_tracing_widget_lib::database::connection::IndexStore;
use token_tracing_widget_lib::sources::session_files::DiscoveryLimits;
use token_tracing_widget_lib::sources::source_config::SourceConfig;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::AppState;
use token_tracing_widget_lib::UsageState;

fn write_profile(include_codex: bool) -> TempDir {
    let profile = tempfile::tempdir().expect("profile should be created");
    let claude_root = profile.path().join(r".claude\projects");
    fs::create_dir_all(&claude_root).expect("Claude root should be created");
    fs::write(
        claude_root.join("session.jsonl"),
        br#"{"message":{"id":"event-synthetic-001","type":"message","usage":{"input_tokens":10,"output_tokens":10}},"sessionId":"session-synthetic-001","timestamp":"2026-01-01T00:00:00Z"}
"#,
    )
    .expect("Claude fixture should be written");

    if include_codex {
        let codex_root = profile.path().join(r".codex\sessions");
        fs::create_dir_all(&codex_root).expect("Codex root should be created");
        fs::write(
            codex_root.join("session.jsonl"),
            br#"{"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":10,"total_tokens":20}}},"timestamp":"2026-01-01T00:00:01Z"}
"#,
        )
        .expect("Codex fixture should be written");
    }

    profile
}

fn limits() -> DiscoveryLimits {
    DiscoveryLimits::new(10, 10_000)
}

#[test]
fn runtime_collects_native_sources_and_returns_post_commit_summary() {
    let profile = write_profile(true);
    let database = tempfile::tempdir().expect("database directory should be created");
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join(r"nested\index.sqlite"),
        limits(),
    )
    .expect("runtime should open");

    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .expect("initial collection should commit");

    assert_eq!(report.summary.today_tokens, 40);
    assert_eq!(report.summary.provider.as_deref(), Some("Codex"));
    assert_eq!(report.summary.state, UsageState::Active);
    assert_eq!(report.accepted_event_count, 2);
}

#[test]
fn runtime_restart_reuses_checkpoints_and_deduplicates_existing_events() {
    let profile = write_profile(true);
    let database = tempfile::tempdir().expect("database directory should be created");
    let database_path = database.path().join("index.sqlite");
    let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");

    let first = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("first runtime should open");
    assert_eq!(first.collect_once(&clock).unwrap().summary.today_tokens, 40);
    drop(first);

    let second = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("restarted runtime should open");
    let report = second.collect_once(&clock).expect("restart should collect");

    assert_eq!(report.summary.today_tokens, 40);
    assert_eq!(report.accepted_event_count, 0);
}

#[test]
fn missing_codex_root_does_not_block_claude_collection() {
    let profile = write_profile(false);
    let database = tempfile::tempdir().expect("database directory should be created");
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        limits(),
    )
    .expect("runtime should open");

    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .expect("Claude collection should succeed");

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.source_health.len(), 2);
    assert_eq!(report.source_health[0].provider, Provider::Claude);
    assert_eq!(report.source_health[0].state, "detected");
    assert_eq!(report.source_health[1].provider, Provider::Codex);
    assert_eq!(report.source_health[1].state, "not_detected");
}

#[test]
fn unavailable_fallback_contains_no_private_fields() {
    let serialized = serde_json::to_value(AppState::unavailable().summary()).unwrap();
    let object = serialized.as_object().unwrap();

    assert_eq!(
        object.get("state").and_then(|value| value.as_str()),
        Some("unavailable")
    );
    assert_eq!(
        object.get("todayTokens").and_then(|value| value.as_u64()),
        Some(0)
    );
    assert!(!object.contains_key("profileRoot"));
    assert!(!object.contains_key("databasePath"));
    assert!(!object.contains_key("rawRecord"));
}

#[test]
fn runtime_loads_persisted_disabled_provider_without_reading_it() {
    let profile = write_profile(true);
    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let mut index = IndexStore::open(&database_path).unwrap();
    index
        .save_source_config(&SourceConfig::try_new(Provider::Codex, false, None).unwrap())
        .unwrap();
    drop(index);

    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .unwrap();

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.source_health[1].state, "disabled");
}

#[test]
fn source_config_update_is_persisted_before_shared_state_changes() {
    let profile = write_profile(false);
    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    let config = SourceConfig::try_new(Provider::Claude, false, None).unwrap();

    state.update_source_config(config.clone()).unwrap();

    assert_eq!(state.source_config(Provider::Claude).unwrap(), config);
    let reopened = IndexStore::open(&database_path).unwrap();
    assert_eq!(
        reopened
            .load_source_configs()
            .unwrap()
            .configs
            .get(Provider::Claude),
        &config
    );
}

#[test]
fn explicit_root_never_enters_summary_payload() {
    let profile = write_profile(false);
    let explicit_root = profile.path().join("private-root");
    fs::create_dir_all(&explicit_root).unwrap();
    fs::copy(
        profile
            .path()
            .join(".claude")
            .join("projects")
            .join("session.jsonl"),
        explicit_root.join("session.jsonl"),
    )
    .unwrap();
    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    let config =
        SourceConfig::try_new(Provider::Claude, true, Some(explicit_root.clone())).unwrap();
    let explicit_label = explicit_root.to_string_lossy().into_owned();
    state.update_source_config(config).unwrap();

    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .unwrap();
    let serialized = serde_json::to_string(&report.summary).unwrap();

    assert_eq!(report.summary.today_tokens, 20);
    assert!(!serialized.contains(&explicit_label));
    assert!(!serialized.contains("rawRecord"));
    assert!(!serialized.contains("working_directory"));
}

#[test]
fn malformed_setting_records_only_a_sanitized_diagnostic_category() {
    let profile = write_profile(false);
    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let index = IndexStore::open(&database_path).unwrap();
    drop(index);
    let connection = rusqlite::Connection::open(&database_path).unwrap();
    connection
        .execute(
            "INSERT INTO settings(setting_key, setting_value) VALUES (?1, ?2)",
            ["source.claude.root_override", "prompt=secret/private-root"],
        )
        .unwrap();
    drop(connection);

    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .unwrap();

    let connection = rusqlite::Connection::open(&database_path).unwrap();
    let category: String = connection
        .query_row(
            "SELECT category FROM diagnostics WHERE provider = ?1",
            ["claude"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(category, "invalid_settings");
}
