//! Explicit user actions. Content edits never promote trust automatically.
use crate::{CommandError, ErrorCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Proposed,
    Confirmed,
    Temporary,
    Durable,
    Superseded,
    Archived,
}

impl MemoryState {
    pub fn allows(self, next: Self) -> bool {
        use MemoryState::*;
        matches!(
            (self, next),
            (Proposed, Confirmed | Temporary | Archived)
                | (
                    Confirmed,
                    Proposed | Temporary | Durable | Superseded | Archived
                )
                | (
                    Temporary,
                    Proposed | Confirmed | Durable | Superseded | Archived
                )
                | (
                    Durable,
                    Proposed | Confirmed | Temporary | Superseded | Archived
                )
                | (Superseded, Archived)
        )
    }
}
pub fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidMemory,
        "Choose an allowed lifecycle action. Temporary requires expiry; supersession requires an existing successor. Archive restoration returns to the previous state.",
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifecycleChange {
    pub state: MemoryState,
    /// Mandatory explicit user consent, including confirmation and retention.
    pub confirmed: bool,
    pub successor_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::MemoryState::*;
    #[test]
    fn all_thirty_six_transitions_are_explicit() {
        let states = [
            Proposed, Confirmed, Temporary, Durable, Superseded, Archived,
        ];
        let expected = [
            vec![Confirmed, Temporary, Archived],
            vec![Proposed, Temporary, Durable, Superseded, Archived],
            vec![Proposed, Confirmed, Durable, Superseded, Archived],
            vec![Proposed, Confirmed, Temporary, Superseded, Archived],
            vec![Archived],
            vec![],
        ];
        for (i, from) in states.iter().enumerate() {
            for to in states {
                assert_eq!(
                    from.allows(to),
                    expected[i].contains(&to),
                    "{from:?}->{to:?}"
                );
            }
        }
    }
}
