use std::fs;
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use token_tracing_widget_lib::collection::{CollectionBatch, FixedClock};
use token_tracing_widget_lib::database::store::IndexStore;
use token_tracing_widget_lib::sources::session_files::DiscoveryLimits;
use token_tracing_widget_lib::sources::source_config::SourceConfig;
use token_tracing_widget_lib::types::file_checkpoint::FileCheckpoint;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::usage_event::UsageEvent;
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
            br#"{"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":10,"total_tokens":20}},"rate_limits":{"primary":{"used_percent":12.0,"window_minutes":300,"resets_at":1788367052},"secondary":{"used_percent":38.0,"window_minutes":10080,"resets_at":1788748134},"plan_type":"plus"}},"timestamp":"2026-01-01T00:00:01Z"}
"#,
        )
        .expect("Codex fixture should be written");
    }

    profile
}

fn limits() -> DiscoveryLimits {
    DiscoveryLimits::new(10, 10_000)
}

fn file_identity(provider: Provider, path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(provider.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(path.to_string_lossy().as_bytes());
    format!("{:x}", hasher.finalize())
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
        .collect_once(&FixedClock::new("2026-01-01T00:00:05Z", "2026-01-01"))
        .expect("initial collection should commit");

    assert_eq!(report.summary.today_tokens, 40);
    assert_eq!(report.summary.provider.as_deref(), Some("Codex"));
    assert_eq!(report.summary.state, UsageState::Active);
    assert_eq!(report.summary.providers[0].provider, Provider::Claude);
    assert_eq!(report.summary.providers[0].today_tokens, 20);
    assert_eq!(report.summary.providers[1].provider, Provider::Codex);
    assert_eq!(report.summary.providers[1].today_tokens, 20);
    assert_eq!(report.summary.providers[1].rate_limits.len(), 2);
    assert_eq!(report.summary.providers[1].rate_limits[0].used_percent, 12);
    assert_eq!(report.accepted_event_count, 2);
}

#[test]
fn codex_session_index_name_updates_without_a_new_token_record() {
    let profile = tempfile::tempdir().unwrap();
    let session_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("01")
        .join("01");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join(format!(
            "rollout-2026-01-01T00-00-00-{session_id}.jsonl"
        )),
        br#"{"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":10,"total_tokens":20}}},"timestamp":"2026-01-01T00:00:01Z"}
"#,
    )
    .unwrap();
    let index_path = profile.path().join(".codex").join("session_index.jsonl");
    fs::write(
        &index_path,
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"First name\",\"updated_at\":\"2026-01-01T00:00:02Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        limits(),
    )
    .unwrap();
    let first = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:05Z", "2026-01-01"))
        .unwrap();
    let first_session = &first.summary.providers[1].sessions[0];
    assert_eq!(first_session.name.as_deref(), Some("First name"));
    assert_eq!(first_session.today_tokens, 20);

    fs::write(
        &index_path,
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"Renamed session\",\"updated_at\":\"2026-01-01T00:00:06Z\"}}\n"
        ),
    )
    .unwrap();
    let second = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .unwrap();
    let second_session = &second.summary.providers[1].sessions[0];
    assert_eq!(second.accepted_event_count, 0);
    assert_eq!(second_session.name.as_deref(), Some("Renamed session"));
    assert_eq!(second_session.today_tokens, 20);
}

#[test]
fn codex_collection_links_rollout_to_parent_session_index_entry() {
    let profile = tempfile::tempdir().unwrap();
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("02");
    fs::create_dir_all(&sessions).unwrap();

    let session_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    let rollout_id = "01a0627e-37d6-79e3-9d4f-cd0a566daddc";
    let file = sessions.join(format!("rollout-2026-09-02T14-20-34-{rollout_id}.jsonl"));
    let metadata = serde_json::json!({
        "payload": {
            "type": "session_meta",
            "id": rollout_id,
            "session_id": session_id,
            "parent_thread_id": session_id,
            "thread_id": rollout_id,
        },
    });
    let token_count = serde_json::json!({
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": {
                    "input_tokens": 10,
                    "output_tokens": 10,
                    "total_tokens": 20,
                },
            },
        },
        "timestamp": "2026-09-02T14:20:35Z",
    });
    fs::write(
        &file,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&metadata).unwrap(),
            serde_json::to_string(&token_count).unwrap()
        ),
    )
    .unwrap();
    fs::write(
        profile.path().join(".codex").join("session_index.jsonl"),
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"Current task\",\"updated_at\":\"2026-09-02T14:20:36Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        limits(),
    )
    .unwrap();
    let report = state
        .collect_once(&FixedClock::new("2026-09-02T14:20:40Z", "2026-09-02"))
        .unwrap();

    let codex = &report.summary.providers[1];
    assert_eq!(codex.today_tokens, 20);
    assert_eq!(codex.sessions.len(), 1);
    assert_eq!(codex.sessions[0].name.as_deref(), Some("Current task"));
    assert_eq!(codex.sessions[0].today_tokens, 20);
}

#[test]
fn codex_rollouts_share_one_logical_session_summary() {
    let profile = tempfile::tempdir().unwrap();
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("02");
    fs::create_dir_all(&sessions).unwrap();

    let session_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    for (rollout_id, total, timestamp, file_timestamp) in [
        (
            "01a0627e-37d6-79e3-9d4f-cd0a566daddc",
            20,
            "14:20:35",
            "14-20-35",
        ),
        (
            "01a0627e-37d6-79e3-9d4f-cd0a566dade",
            30,
            "14:21:35",
            "14-21-35",
        ),
    ] {
        let metadata = serde_json::json!({
            "payload": {
                "type": "session_meta",
                "session_id": session_id,
                "parent_thread_id": session_id,
            },
        });
        let token_count = serde_json::json!({
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 10,
                        "output_tokens": total - 10,
                        "total_tokens": total,
                    },
                },
            },
            "timestamp": format!("2026-09-02T{timestamp}Z"),
        });
        fs::write(
            sessions.join(format!(
                "rollout-2026-09-02T{file_timestamp}-{rollout_id}.jsonl"
            )),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&metadata).unwrap(),
                serde_json::to_string(&token_count).unwrap()
            ),
        )
        .unwrap();
    }
    fs::write(
        profile.path().join(".codex").join("session_index.jsonl"),
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"One session\",\"updated_at\":\"2026-09-02T14:22:00Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        limits(),
    )
    .unwrap();
    let report = state
        .collect_once(&FixedClock::new("2026-09-02T14:23:00Z", "2026-09-02"))
        .unwrap();

    let codex = &report.summary.providers[1];
    assert_eq!(codex.today_tokens, 50);
    assert_eq!(codex.sessions.len(), 1);
    assert_eq!(codex.sessions[0].name.as_deref(), Some("One session"));
    assert_eq!(codex.sessions[0].today_tokens, 50);
}

#[test]
fn codex_collection_rekeys_legacy_rollout_sessions() {
    let profile = tempfile::tempdir().unwrap();
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("02");
    fs::create_dir_all(&sessions).unwrap();

    let session_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    let mut legacy_events = Vec::new();
    let mut legacy_checkpoints = Vec::new();
    for (rollout_id, total, timestamp, file_timestamp) in [
        (
            "01a0627e-37d6-79e3-9d4f-cd0a566daddc",
            20,
            "14:20:35",
            "14-20-35",
        ),
        (
            "01a0627e-37d6-79e3-9d4f-cd0a566dade",
            30,
            "14:21:35",
            "14-21-35",
        ),
    ] {
        let file = sessions.join(format!(
            "rollout-2026-09-02T{file_timestamp}-{rollout_id}.jsonl"
        ));
        let metadata = serde_json::json!({
            "payload": {
                "type": "session_meta",
                "session_id": session_id,
                "parent_thread_id": session_id,
            },
        });
        let token_count = serde_json::json!({
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 10,
                        "output_tokens": total - 10,
                        "total_tokens": total,
                    },
                },
            },
            "timestamp": format!("2026-09-02T{timestamp}Z"),
        });
        fs::write(
            &file,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&metadata).unwrap(),
                serde_json::to_string(&token_count).unwrap()
            ),
        )
        .unwrap();
        let identity = file_identity(Provider::Codex, &file);
        let size = fs::metadata(&file).unwrap().len();
        legacy_events.push(UsageEvent::for_test(
            Provider::Codex,
            &identity,
            &format!("2026-09-02T{timestamp}Z"),
            total,
        ));
        legacy_checkpoints.push(FileCheckpoint::with_position(
            identity,
            Provider::Codex,
            size,
            size,
        ));
    }
    fs::write(
        profile.path().join(".codex").join("session_index.jsonl"),
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"One session\",\"updated_at\":\"2026-09-02T14:22:00Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let mut seeded_database = IndexStore::open(&database_path).unwrap();
    seeded_database
        .apply_batch(&CollectionBatch::new(legacy_events, legacy_checkpoints))
        .unwrap();
    drop(seeded_database);

    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    let report = state
        .collect_once(&FixedClock::new("2026-09-02T14:23:00Z", "2026-09-02"))
        .unwrap();

    let codex = &report.summary.providers[1];
    assert_eq!(codex.today_tokens, 50);
    assert_eq!(codex.sessions.len(), 1);
    assert_eq!(codex.sessions[0].name.as_deref(), Some("One session"));
    assert_eq!(codex.sessions[0].today_tokens, 50);
}

#[test]
fn codex_collection_excludes_sessions_without_index_entry() {
    let profile = tempfile::tempdir().unwrap();
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("02");
    fs::create_dir_all(&sessions).unwrap();

    let indexed_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    let unindexed_id = "019feeb0-0072-75a1-8d25-010d8bb342c9";
    for (session_id, total, primary_used, secondary_used, timestamp) in [
        (indexed_id, 20, 16, 66, "2026-09-02T00:00:01Z"),
        (unindexed_id, 30, 25, 67, "2026-09-02T00:00:04Z"),
    ] {
        let record = serde_json::json!({
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 10,
                        "output_tokens": total - 10,
                        "total_tokens": total,
                    },
                },
                "rate_limits": {
                    "primary": {
                        "used_percent": primary_used as f64,
                        "window_minutes": 300,
                        "resets_at": 1788367052_u64,
                    },
                    "secondary": {
                        "used_percent": secondary_used as f64,
                        "window_minutes": 10080,
                        "resets_at": 1788748134_u64,
                    },
                },
            },
            "timestamp": timestamp,
        });
        fs::write(
            sessions.join(format!("rollout-2026-09-02T00-00-00-{session_id}.jsonl")),
            format!("{}\n", serde_json::to_string(&record).unwrap()),
        )
        .unwrap();
    }
    let previous_day_sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("01");
    fs::create_dir_all(&previous_day_sessions).unwrap();
    let previous_day_id = "019feeb0-0072-75a1-8d25-010d8bb342ca";
    fs::write(
        previous_day_sessions.join(format!(
            "rollout-2026-09-01T23-59-59-{previous_day_id}.jsonl"
        )),
        br#"{"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"output_tokens":30,"total_tokens":40}}},"timestamp":"2026-09-02T00:00:01Z"}
"#,
    )
    .unwrap();
    fs::write(
        profile.path().join(".codex").join("session_index.jsonl"),
        format!(
            "{{\"id\":\"{indexed_id}\",\"thread_name\":\"Indexed session\",\"updated_at\":\"2026-09-02T00:00:02Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let database_path = database.path().join("index.sqlite");
    let mut seeded_database = IndexStore::open(&database_path).unwrap();
    seeded_database
        .apply_batch(&CollectionBatch::new(
            vec![UsageEvent::for_test(
                Provider::Codex,
                "stale-file-identity",
                "2026-09-02T00:00:03Z",
                99,
            )],
            Vec::new(),
        ))
        .unwrap();
    drop(seeded_database);
    let state =
        AppState::from_paths(profile.path().to_path_buf(), &database_path, limits()).unwrap();
    let report = state
        .collect_once(&FixedClock::new("2026-09-02T00:00:05Z", "2026-09-02"))
        .unwrap();

    let codex = &report.summary.providers[1];
    assert_eq!(codex.today_tokens, 20);
    assert_eq!(codex.sessions.len(), 1);
    assert_eq!(codex.sessions[0].name.as_deref(), Some("Indexed session"));
    assert_eq!(codex.rate_limits.len(), 2);
    assert_eq!(codex.rate_limits[0].used_percent, 25);
    assert_eq!(codex.rate_limits[1].used_percent, 67);
}

#[test]
fn codex_collection_reads_today_append_when_session_index_has_previous_day_timestamp() {
    let profile = tempfile::tempdir().unwrap();
    let session_id = "019feeb0-0072-75a1-8d25-010d8bb342c8";
    let sessions = profile
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("01");
    fs::create_dir_all(&sessions).unwrap();
    let file = sessions.join(format!("rollout-2026-09-01T00-00-00-{session_id}.jsonl"));
    let metadata = serde_json::json!({
        "payload": {
            "type": "session_meta",
            "session_id": session_id,
            "parent_thread_id": session_id,
        },
    });
    let yesterday = serde_json::json!({
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": {
                    "input_tokens": 5,
                    "output_tokens": 5,
                    "total_tokens": 10,
                },
            },
        },
        "timestamp": "2026-09-01T00:00:01Z",
    });
    fs::write(
        &file,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&metadata).unwrap(),
            serde_json::to_string(&yesterday).unwrap()
        ),
    )
    .unwrap();
    let index_path = profile.path().join(".codex").join("session_index.jsonl");
    fs::write(
        &index_path,
        format!(
            "{{\"id\":\"{session_id}\",\"thread_name\":\"Overnight run\",\"updated_at\":\"2026-09-01T00:00:02Z\"}}\n"
        ),
    )
    .unwrap();

    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        limits(),
    )
    .unwrap();
    let yesterday_report = state
        .collect_once(&FixedClock::new("2026-09-01T00:00:05Z", "2026-09-01"))
        .unwrap();
    assert_eq!(yesterday_report.summary.today_tokens, 10);

    let today = serde_json::json!({
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": {
                    "input_tokens": 15,
                    "output_tokens": 15,
                    "total_tokens": 30,
                },
            },
        },
        "timestamp": "2026-09-02T00:00:01Z",
    });
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(format!("{}\n", serde_json::to_string(&today).unwrap()).as_bytes())
        .unwrap();

    let report = state
        .collect_once(&FixedClock::new("2026-09-02T00:00:05Z", "2026-09-02"))
        .unwrap();

    let codex = &report.summary.providers[1];
    assert_eq!(report.accepted_event_count, 1);
    assert_eq!(codex.today_tokens, 20);
    assert_eq!(codex.sessions.len(), 1);
    assert_eq!(codex.sessions[0].today_tokens, 20);
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
