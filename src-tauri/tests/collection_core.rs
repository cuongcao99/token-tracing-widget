use token_tracing_widget_lib::providers::provider_adapter::ProviderReadObservation;
use token_tracing_widget_lib::types::file_checkpoint::FileCheckpoint;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::token_observation::{CounterKind, TokenObservation};
use token_tracing_widget_lib::usage::cumulative_delta::convert_observations;
use token_tracing_widget_lib::usage::observation_validation::{
    validate_observation, ObservationValidationError,
};

fn codex_observation(timestamp: &str, total: u64) -> TokenObservation {
    let input_tokens = total / 2;
    TokenObservation {
        provider: Provider::Codex,
        source_session_key: None,
        source_event_key: None,
        observed_at: timestamp.to_owned(),
        counter_kind: CounterKind::Cumulative,
        input_tokens: Some(input_tokens),
        cached_input_tokens: Some(total),
        output_tokens: Some(total - input_tokens),
        total_tokens: total,
    }
}

#[test]
fn cumulative_snapshots_become_deltas_and_reset_starts_new_segment() {
    let observations = vec![
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:00Z", 10), 0),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:01Z", 20), 100),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:02Z", 5), 200),
    ];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Codex);
    let batch = convert_observations("file-a", &checkpoint, observations).unwrap();

    assert_eq!(
        batch
            .events
            .iter()
            .map(|event| event.total_tokens)
            .collect::<Vec<_>>(),
        vec![10, 10, 5]
    );
    assert_eq!(batch.events[0].monotonic_segment, 0);
    assert_eq!(batch.events[2].monotonic_segment, 1);
}

#[test]
fn duplicate_stable_event_key_is_accepted_once() {
    let observation = TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: Some("event-1".to_owned()),
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    };
    let observations = vec![ProviderReadObservation::new(observation, 0)];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Claude);
    let first = convert_observations("file-a", &checkpoint, observations.clone()).unwrap();
    let second = convert_observations("file-a", &first.next_checkpoint, observations).unwrap();

    assert_eq!(first.events.len(), 1);
    assert!(second.events.is_empty());
}

#[test]
fn duplicate_stable_event_key_is_rejected_even_at_a_new_source_position() {
    let observation = TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: Some("event-1".to_owned()),
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    };
    let first = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Claude),
        vec![ProviderReadObservation::new(observation.clone(), 0)],
    )
    .unwrap();
    let second = convert_observations(
        "file-a",
        &first.next_checkpoint,
        vec![ProviderReadObservation::new(observation, 100)],
    )
    .unwrap();

    assert!(second.events.is_empty());
}

fn incremental_observation(
    event_key: Option<&str>,
    timestamp: &str,
    input_tokens: u64,
    output_tokens: u64,
) -> TokenObservation {
    TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: event_key.map(str::to_owned),
        observed_at: timestamp.to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(input_tokens),
        cached_input_tokens: None,
        output_tokens: Some(output_tokens),
        total_tokens: input_tokens + output_tokens,
    }
}

#[test]
fn observations_are_ordered_by_timestamp_then_source_position() {
    let observations = vec![
        ProviderReadObservation::new(
            incremental_observation(Some("late"), "2026-01-01T00:00:01Z", 2, 3),
            20,
        ),
        ProviderReadObservation::new(
            incremental_observation(Some("early"), "2026-01-01T00:00:00Z", 4, 6),
            10,
        ),
        ProviderReadObservation::new(
            incremental_observation(Some("same-time"), "2026-01-01T00:00:01Z", 1, 1),
            15,
        ),
    ];
    let batch = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Claude),
        observations,
    )
    .unwrap();

    let keys: Vec<_> = batch
        .events
        .iter()
        .map(|event| event.observed_at.as_str())
        .collect();
    assert_eq!(
        keys,
        vec![
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:01Z",
            "2026-01-01T00:00:01Z"
        ]
    );
    assert_eq!(batch.events[1].source_position, 15);
    assert_eq!(batch.events[2].source_position, 20);
}

#[test]
fn missing_source_session_key_falls_back_to_opaque_file_identity() {
    let batch = convert_observations(
        "opaque-file-id",
        &FileCheckpoint::new("opaque-file-id", Provider::Codex),
        vec![ProviderReadObservation::new(
            codex_observation("2026-01-01T00:00:00Z", 10),
            0,
        )],
    )
    .unwrap();

    assert_eq!(batch.events[0].session_key, "opaque-file-id");
    assert!(!batch.events[0].event_id.contains("opaque-file-id"));
}

#[test]
fn validation_rejects_inconsistent_total_and_checked_add_overflow() {
    let inconsistent = incremental_observation(Some("bad-total"), "2026-01-01T00:00:00Z", 10, 5);
    assert_eq!(
        validate_observation(&TokenObservation {
            total_tokens: 20,
            ..inconsistent
        }),
        Err(ObservationValidationError::TotalMismatch)
    );

    let overflowing = TokenObservation {
        input_tokens: Some(u64::MAX),
        output_tokens: Some(1),
        total_tokens: 0,
        ..incremental_observation(Some("overflow"), "2026-01-01T00:00:00Z", 0, 0)
    };
    assert_eq!(
        validate_observation(&overflowing),
        Err(ObservationValidationError::CounterOverflow)
    );
}

#[test]
fn cached_input_changes_do_not_inflate_total_delta() {
    let first = TokenObservation {
        cached_input_tokens: Some(2),
        ..codex_observation("2026-01-01T00:00:00Z", 10)
    };
    let second = TokenObservation {
        cached_input_tokens: Some(7),
        ..codex_observation("2026-01-01T00:00:01Z", 10)
    };
    let batch = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Codex),
        vec![
            ProviderReadObservation::new(first, 0),
            ProviderReadObservation::new(second, 100),
        ],
    )
    .unwrap();

    assert_eq!(batch.events[0].total_tokens, 10);
    assert_eq!(batch.events[1].total_tokens, 0);
    assert_eq!(batch.events[1].cached_input_tokens, Some(5));
    assert_eq!(
        batch
            .events
            .iter()
            .map(|event| event.total_tokens)
            .sum::<u64>(),
        10
    );
}
