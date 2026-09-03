//! Converting cumulative counters into incremental token deltas.

use std::fmt;

use crate::providers::provider_adapter::ProviderReadObservation;
use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::token_observation::{CounterKind, TokenObservation};
use crate::types::usage_event::UsageEvent;
use crate::usage::duplicate_event_filter::{effective_session_key, event_id};
use crate::usage::observation_validation::{validate_observation, ObservationValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaBatch {
    pub events: Vec<UsageEvent>,
    pub next_checkpoint: FileCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaConversionError {
    Validation(ObservationValidationError),
    ProviderMismatch,
}

impl fmt::Display for DeltaConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(formatter, "validation:{error}"),
            Self::ProviderMismatch => formatter.write_str("provider_mismatch"),
        }
    }
}

impl std::error::Error for DeltaConversionError {}

impl From<ObservationValidationError> for DeltaConversionError {
    fn from(error: ObservationValidationError) -> Self {
        Self::Validation(error)
    }
}

pub fn convert_observations(
    file_identity: &str,
    checkpoint: &FileCheckpoint,
    mut observations: Vec<ProviderReadObservation>,
) -> Result<DeltaBatch, DeltaConversionError> {
    if observations
        .iter()
        .any(|entry| entry.observation.provider != checkpoint.provider)
    {
        return Err(DeltaConversionError::ProviderMismatch);
    }

    observations.sort_by(|left, right| {
        left.observation
            .observed_at
            .cmp(&right.observation.observed_at)
            .then_with(|| left.source_position.cmp(&right.source_position))
    });

    let mut next_checkpoint = checkpoint.clone();
    next_checkpoint.file_identity = file_identity.to_owned();
    let mut events = Vec::new();
    let mut seen_ids: std::collections::HashSet<_> =
        checkpoint.seen_event_ids.iter().cloned().collect();

    for entry in observations {
        if entry.source_position < checkpoint.byte_offset {
            continue;
        }

        let observation = &entry.observation;
        validate_observation(observation)?;
        let id = event_id(
            observation.provider,
            file_identity,
            observation,
            entry.source_position,
        );
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        next_checkpoint.seen_event_ids.push(id.clone());

        let (input_tokens, cached_input_tokens, output_tokens, total_tokens, segment) =
            match observation.counter_kind {
                CounterKind::Incremental => (
                    observation.input_tokens,
                    observation.cached_input_tokens,
                    observation.output_tokens,
                    observation.total_tokens,
                    next_checkpoint.monotonic_segment,
                ),
                CounterKind::Cumulative => {
                    let reset = cumulative_decreased(&next_checkpoint, observation);
                    if reset {
                        next_checkpoint.monotonic_segment =
                            next_checkpoint.monotonic_segment.saturating_add(1);
                    }

                    let deltas = cumulative_deltas(&next_checkpoint, observation, reset);
                    next_checkpoint.last_cumulative_input_tokens = observation.input_tokens;
                    next_checkpoint.last_cumulative_cached_input_tokens =
                        observation.cached_input_tokens;
                    next_checkpoint.last_cumulative_output_tokens = observation.output_tokens;
                    next_checkpoint.last_cumulative_total_tokens = Some(observation.total_tokens);
                    (
                        deltas.0,
                        deltas.1,
                        deltas.2,
                        deltas.3,
                        next_checkpoint.monotonic_segment,
                    )
                }
            };

        next_checkpoint.byte_offset = next_checkpoint
            .byte_offset
            .max(entry.source_position.saturating_add(1));

        if has_tokens(
            input_tokens,
            cached_input_tokens,
            output_tokens,
            total_tokens,
        ) {
            events.push(UsageEvent::from_delta(
                id,
                file_identity.to_owned(),
                entry.source_position,
                observation,
                effective_session_key(observation, file_identity),
                segment,
                input_tokens,
                cached_input_tokens,
                output_tokens,
                total_tokens,
            ));
        }
    }

    const MAX_RETAINED_EVENT_IDS: usize = 4_096;
    if next_checkpoint.seen_event_ids.len() > MAX_RETAINED_EVENT_IDS {
        let remove_count = next_checkpoint.seen_event_ids.len() - MAX_RETAINED_EVENT_IDS;
        next_checkpoint.seen_event_ids.drain(..remove_count);
    }

    Ok(DeltaBatch {
        events,
        next_checkpoint,
    })
}

fn cumulative_decreased(checkpoint: &FileCheckpoint, observation: &TokenObservation) -> bool {
    decreases(
        observation.input_tokens,
        checkpoint.last_cumulative_input_tokens,
    ) || decreases(
        observation.cached_input_tokens,
        checkpoint.last_cumulative_cached_input_tokens,
    ) || decreases(
        observation.output_tokens,
        checkpoint.last_cumulative_output_tokens,
    ) || decreases(
        Some(observation.total_tokens),
        checkpoint.last_cumulative_total_tokens,
    )
}

fn decreases(current: Option<u64>, previous: Option<u64>) -> bool {
    matches!((current, previous), (Some(current), Some(previous)) if current < previous)
}

fn cumulative_deltas(
    checkpoint: &FileCheckpoint,
    observation: &TokenObservation,
    reset: bool,
) -> (Option<u64>, Option<u64>, Option<u64>, u64) {
    (
        counter_delta(
            observation.input_tokens,
            checkpoint.last_cumulative_input_tokens,
            reset,
        ),
        counter_delta(
            observation.cached_input_tokens,
            checkpoint.last_cumulative_cached_input_tokens,
            reset,
        ),
        counter_delta(
            observation.output_tokens,
            checkpoint.last_cumulative_output_tokens,
            reset,
        ),
        counter_delta(
            Some(observation.total_tokens),
            checkpoint.last_cumulative_total_tokens,
            reset,
        )
        .unwrap_or(observation.total_tokens),
    )
}

fn counter_delta(current: Option<u64>, previous: Option<u64>, reset: bool) -> Option<u64> {
    match (current, previous, reset) {
        (None, _, _) => None,
        (Some(current), _, true) => Some(current),
        (Some(current), Some(previous), false) => current.checked_sub(previous),
        (Some(current), None, false) => Some(current),
    }
}

fn has_tokens(
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: u64,
) -> bool {
    total_tokens > 0
        || input_tokens.is_some_and(|value| value > 0)
        || cached_input_tokens.is_some_and(|value| value > 0)
        || output_tokens.is_some_and(|value| value > 0)
}
