//! Named visual systems supported by the presentation surfaces.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Claude,
}

impl Theme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}
