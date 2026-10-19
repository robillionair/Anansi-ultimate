//! Reject duplicate keys at every nesting level, including user metadata maps.
use super::{Result, VaultError};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt};
use zeroize::Zeroize;
struct Unique;
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Walk;
        impl<'de> Visitor<'de> for Walk {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Unique, M::Error> {
                let mut seen = HashSet::<[u8; 32]>::new();
                while let Some(mut key) = map.next_key::<String>()? {
                    let digest = Sha256::digest(key.as_bytes()).into();
                    key.zeroize();
                    if !seen.insert(digest) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    map.next_value::<Unique>()?;
                }
                Ok(Unique)
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> std::result::Result<Unique, S::Error> {
                while seq.next_element::<Unique>()?.is_some() {}
                Ok(Unique)
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_string<E: serde::de::Error>(
                self,
                mut value: String,
            ) -> std::result::Result<Unique, E> {
                value.zeroize();
                Ok(Unique)
            }
        }
        d.deserialize_any(Walk)
    }
}
pub(crate) fn validate(bytes: &[u8], maximum: usize) -> Result<()> {
    if bytes.len() > maximum {
        return Err(VaultError::ResourceLimit);
    }
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    Unique::deserialize(&mut parser).map_err(|_| VaultError::InvalidEnvelope)?;
    parser.end().map_err(|_| VaultError::InvalidEnvelope)
}
#[cfg(test)]
mod tests {
    #[test]
    fn nested_duplicate_and_trailing_data_refused() {
        assert!(super::validate(br#"{"metadata":{"x":1,"x":2}}"#, 1024).is_err());
        assert!(super::validate(br#"{"metadata":{"x":1},"array":[{"x":2}]}"#, 1024).is_ok());
        assert!(super::validate(b"{} {}", 1024).is_err());
    }
}
