//! Validating normalized observations before aggregation.

use std::fmt;

use crate::types::token_observation::TokenObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationValidationError {
    MissingTimestamp,
    TotalMismatch,
    CounterOverflow,
}

impl fmt::Display for ObservationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::MissingTimestamp => "missing_timestamp",
            Self::TotalMismatch => "total_mismatch",
            Self::CounterOverflow => "counter_overflow",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for ObservationValidationError {}

pub fn validate_observation(
    observation: &TokenObservation,
) -> Result<(), ObservationValidationError> {
    if observation.observed_at.trim().is_empty() {
        return Err(ObservationValidationError::MissingTimestamp);
    }

    if let (Some(input_tokens), Some(output_tokens)) =
        (observation.input_tokens, observation.output_tokens)
    {
        let expected_total = input_tokens
            .checked_add(output_tokens)
            .ok_or(ObservationValidationError::CounterOverflow)?;
        if observation.total_tokens != expected_total {
            return Err(ObservationValidationError::TotalMismatch);
        }
    }

    Ok(())
}
