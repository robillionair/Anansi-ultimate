//! Vault-owned filtering boundary. Search integration is deliberately deferred.
use super::session::{Caller, Sessions};
use super::{Result, VaultError};
use crate::privacy::Privacy;
use std::time::Instant;

#[derive(Clone, Copy)]
pub enum Operation {
    List,
    Read,
    History,
    Relationship,
    Edit,
    OrdinaryExport,
    EncryptedBackup,
}

pub fn allow(privacy: Privacy, operation: Operation, authorized: bool) -> Result<()> {
    if privacy != Privacy::Sealed {
        return Ok(());
    }
    // Ciphertext backup publication still requires manifest/member validation.
    if matches!(operation, Operation::EncryptedBackup) {
        return Ok(());
    }
    if matches!(operation, Operation::OrdinaryExport) || !authorized {
        return Err(VaultError::Locked);
    }
    Ok(())
}

/// Future Search owner must filter before indexing/counting and revalidate on
/// publication. This does not itself grant a session or return decrypted metadata.
pub trait SearchVisibility {
    fn may_index(&self, privacy: Privacy) -> bool;
    fn recheck_publication(&self) -> Result<()>;
}

/// Vault-owned search boundary for the application integration layer.
///
/// Search may build an index only from records that are already known to be
/// non-Sealed. If a request is authorized to inspect Sealed metadata, the
/// integration must call `publish` immediately before returning results; the
/// session check revalidates caller identity, idle/absolute deadlines, and the
/// durable generation epoch. This interface intentionally returns no key or
/// decrypted payload.
pub struct SearchBoundary;

impl SearchBoundary {
    pub fn may_index(privacy: Privacy) -> bool {
        privacy != Privacy::Sealed
    }

    pub fn publish<T>(
        sessions: &mut Sessions,
        caller: &Caller,
        token: &[u8; 16],
        epoch: u64,
        now: Instant,
        privacy: Privacy,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        if privacy != Privacy::Sealed {
            return operation();
        }
        sessions.authorize(caller, token, epoch, now, |_| operation())
    }

    /// Remove plaintext FTS rows before a Sealed transition or generation
    /// revocation becomes visible. The caller must execute this on the same
    /// transaction that changes the authoritative privacy/generation state.
    pub fn invalidate_record(
        connection: &rusqlite::Connection,
        workspace_id: &str,
        record_id: &str,
    ) -> rusqlite::Result<usize> {
        connection.execute(
            "DELETE FROM memory_body_fts WHERE workspace_id=?1 AND memory_id=?2",
            rusqlite::params![workspace_id, record_id],
        )
    }

    /// Remove every plaintext FTS row for a workspace during sealing,
    /// revocation, restore replacement, or a privacy-safe rebuild.
    pub fn invalidate_workspace(
        connection: &rusqlite::Connection,
        workspace_id: &str,
    ) -> rusqlite::Result<usize> {
        connection.execute(
            "DELETE FROM memory_body_fts WHERE workspace_id=?1",
            [workspace_id],
        )
    }
}

/// The only current index policy: Sealed is excluded even if another caller unlocks.
pub struct ExcludeSealed;
impl SearchVisibility for ExcludeSealed {
    fn may_index(&self, privacy: Privacy) -> bool {
        privacy != Privacy::Sealed
    }
    fn recheck_publication(&self) -> Result<()> {
        Ok(())
    }
}
