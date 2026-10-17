use crate::{CommandError, ErrorCode};

pub const SUPPORTED_KINDS: &[&str] = &[
    "memory",
    "note",
    "project",
    "task",
    "person",
    "organization",
    "decision",
    "event",
    "document",
];

pub fn is_valid_kind(kind: &str) -> bool {
    SUPPORTED_KINDS.contains(&kind)
}

pub fn validate_entity_fields(
    kind: &str,
    project_id: Option<&str>,
    due_date: Option<&str>,
    event_date: Option<&str>,
) -> Result<(), CommandError> {
    if !is_valid_kind(kind) {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Invalid entity kind.",
        ));
    }

    if project_id.is_some_and(|pid| pid.is_empty() || pid.len() > 64) {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Project ID must be between 1 and 64 characters.",
        ));
    }

    if due_date.is_some_and(|due| chrono::DateTime::parse_from_rfc3339(due).is_err()) {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Due date must be a valid RFC3339 timestamp.",
        ));
    }

    if event_date.is_some_and(|event| chrono::DateTime::parse_from_rfc3339(event).is_err()) {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Event date must be a valid RFC3339 timestamp.",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_kinds() {
        for kind in SUPPORTED_KINDS {
            assert!(is_valid_kind(kind));
            assert!(validate_entity_fields(kind, None, None, None).is_ok());
        }
        assert!(!is_valid_kind("unsupported_kind"));
        assert!(validate_entity_fields("unsupported_kind", None, None, None).is_err());
    }

    #[test]
    fn validates_task_due_date() {
        assert!(validate_entity_fields("task", None, Some("2026-10-20T12:00:00Z"), None).is_ok());
        assert!(validate_entity_fields("task", None, Some("invalid-date"), None).is_err());
    }
}
