use crate::{CommandError, ErrorCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Privacy {
    Sealed,
    #[default]
    Private,
    Standard,
    Public,
}
impl Privacy {
    pub fn allow_plaintext(self) -> Result<(), CommandError> {
        if self == Self::Sealed {
            Err(CommandError::new(
                ErrorCode::VaultUnavailable,
                "Sealed memory requires the future encrypted vault. No plaintext fallback was used.",
            ))
        } else {
            Ok(())
        }
    }
    pub fn downgrade_from(self, old: Self) -> bool {
        fn rank(p: Privacy) -> u8 {
            match p {
                Privacy::Sealed => 0,
                Privacy::Private => 1,
                Privacy::Standard => 2,
                Privacy::Public => 3,
            }
        }
        rank(self) > rank(old)
    }
}
