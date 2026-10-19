//! Vault-owned filtering boundary. Search integration is deliberately deferred.
use super::{Result, VaultError};
use crate::privacy::Privacy;

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
