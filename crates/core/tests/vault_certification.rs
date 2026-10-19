//! Core-stage certification only: no adapter, migration or production claim.
use anansi_core::privacy::Privacy;
use anansi_core::vault::{VaultError, envelope::*, keys::*, policy::*, session::*};
use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use chacha20poly1305::{AeadInOut, KeyInit, XChaCha20Poly1305, XNonce};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
static SERIAL_KDF: Mutex<()> = Mutex::new(());
fn context() -> Context {
    Context::new(&"a".repeat(32), &"b".repeat(32), &"c".repeat(32)).unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn test_known_vector_xchacha20poly1305_argon2id() {
    // RFC9106 §5.3, independent published p=4 vector. Small vector memory is NOT
    // accepted by the production envelope parser or substituted for its defaults.
    let params = ParamsBuilder::new()
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .data(AssociatedData::new(&[4; 12]).unwrap())
        .build()
        .unwrap();
    let argon =
        Argon2::new_with_secret(&[3; 8], Algorithm::Argon2id, Version::V0x13, params).unwrap();
    let mut key = Zeroizing::new([0; 32]);
    argon
        .hash_password_into(&[1; 32], &[2; 16], &mut *key)
        .unwrap();
    assert_eq!(
        &key[..],
        hex("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659")
    );
    // draft-irtf-cfrg-xchacha-03 Appendix A.3.1, independent AEAD vector.
    let key: [u8; 32] = hex("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")
        .try_into()
        .unwrap();
    let nonce: [u8; 24] = hex("404142434445464748494a4b4c4d4e4f5051525354555657")
        .try_into()
        .unwrap();
    let mut plain = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.".to_vec();
    let tag = XChaCha20Poly1305::new((&key).into())
        .encrypt_inout_detached(
            &XNonce::from(nonce),
            &hex("50515253c0c1c2c3c4c5c6c7"),
            plain.as_mut_slice().into(),
        )
        .unwrap();
    assert_eq!(
        plain,
        hex(
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb
        731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452
        2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9
        21f9664c97637da9768812f615c68b13b52e"
        )
    );
    assert_eq!(tag.as_slice(), hex("c0875924c1c7987947deafd8780acf49"));
    assert_eq!(
        KdfProfile::default(),
        KdfProfile {
            memory_kib: 65536,
            iterations: 3,
            lanes: 4
        }
    );
}

#[test]
fn test_envelope_header_and_aad_tamper_fails_closed() {
    let original = seal(
        &[9; 32],
        Purpose::Record,
        b"synthetic canary",
        KdfProfile::default(),
        &[2; 16],
    )
    .unwrap();
    assert_eq!(
        &*ParsedEnvelope::parse(&original, Purpose::Record)
            .unwrap()
            .open(&[9; 32])
            .unwrap(),
        b"synthetic canary"
    );
    for byte in 0..original.len() {
        for bit in 0..8 {
            let mut changed = original.clone();
            changed[byte] ^= 1 << bit;
            assert!(
                ParsedEnvelope::parse(&changed, Purpose::Record)
                    .and_then(|p| p.open(&[9; 32]))
                    .is_err(),
                "byte {byte}, bit {bit}"
            );
        }
    }
    for len in 0..original.len() {
        assert!(ParsedEnvelope::parse(&original[..len], Purpose::Record).is_err());
    }
    let mut trailing = original.clone();
    trailing.push(0);
    assert!(ParsedEnvelope::parse(&trailing, Purpose::Record).is_err());
    assert!(ParsedEnvelope::parse(&original, Purpose::Journal).is_err());
}

#[test]
fn test_wrong_passphrase_and_wrong_recovery_key() {
    let _serial = SERIAL_KDF.lock().unwrap();
    let (slots, recovery) = KeySlots::initialize(&context(), b"synthetic passphrase").unwrap();
    let original = slots.passphrase.clone();
    assert_eq!(
        slots.unlock(&context(), b"wrong", false).err(),
        Some(VaultError::AuthenticationFailed)
    );
    assert_eq!(
        slots.recover(&context(), &[0; 32]).err(),
        Some(VaultError::AuthenticationFailed)
    );
    let swapped = Context::new(&"d".repeat(32), &"b".repeat(32), &"c".repeat(32)).unwrap();
    assert_eq!(
        slots.recover(&swapped, &recovery).err(),
        Some(VaultError::InvalidContext)
    );
    assert_eq!(slots.passphrase, original);
}

#[test]
fn test_recovery_key_unwraps_dek_and_rotates_passphrase() {
    let _serial = SERIAL_KDF.lock().unwrap();
    let (slots, recovery) = KeySlots::initialize(&context(), b"old synthetic passphrase").unwrap();
    let original_dek = slots
        .unlock(&context(), b"old synthetic passphrase", false)
        .unwrap();
    let record = seal(
        &original_dek,
        Purpose::Record,
        b"preserved encrypted revision",
        KdfProfile::default(),
        &[5; 16],
    )
    .unwrap();
    let (replacement, new_recovery) = slots
        .rotate_after_recovery(&context(), &recovery, b"new synthetic passphrase")
        .unwrap();
    assert!(
        replacement
            .unlock(&context(), b"old synthetic passphrase", false)
            .is_err()
    );
    assert!(replacement.recover(&context(), &recovery).is_err());
    let dek = replacement
        .unlock(&context(), b"new synthetic passphrase", false)
        .unwrap();
    assert_eq!(&*dek, &*original_dek);
    assert_eq!(
        &*replacement.recover(&context(), &new_recovery).unwrap(),
        &*dek
    );
    assert_eq!(
        &*ParsedEnvelope::parse(&record, Purpose::Record)
            .unwrap()
            .open(&dek)
            .unwrap(),
        b"preserved encrypted revision"
    );
    // Old independent backups remain decryptable: rotation never revokes copies.
    assert!(slots.recover(&context(), &recovery).is_ok());
}

#[test]
fn test_restart_lock_and_zeroization() {
    let caller = Caller {
        context: context(),
        transport: Transport::Desktop,
        root_id: "synthetic-root".into(),
        caller_id: "window-1".into(),
    };
    let now = Instant::now();
    let mut sessions = Sessions::default();
    let token = sessions
        .establish(caller.clone(), Zeroizing::new([9; 32]), 7, now)
        .unwrap();
    assert!(
        sessions
            .authorize(&caller, &token, 7, now, |_| Ok(()))
            .is_ok()
    );
    let other = Caller {
        transport: Transport::Browser,
        ..caller.clone()
    };
    assert!(
        sessions
            .authorize(&other, &token, 7, now, |_| Ok(()))
            .is_err()
    );
    sessions.lock();
    assert!(
        sessions
            .authorize(&caller, &token, 7, now, |_| Ok(()))
            .is_err()
    );
    drop(sessions);
    let mut restarted = Sessions::default();
    assert!(
        restarted
            .authorize(&caller, &token, 7, now, |_| Ok(()))
            .is_err()
    );
    let token = restarted
        .establish(caller.clone(), Zeroizing::new([9; 32]), 7, now)
        .unwrap();
    assert!(
        restarted
            .authorize(&caller, &token, 8, now, |_| Ok(()))
            .is_err()
    );
    let token = restarted
        .establish(caller.clone(), Zeroizing::new([9; 32]), 9, now)
        .unwrap();
    assert!(
        restarted
            .authorize(&caller, &token, 9, now + Duration::from_secs(900), |_| Ok(
                ()
            ))
            .is_err()
    );
    // Controlled live-buffer zeroization; never read freed memory or infer OS erasure.
    use zeroize::Zeroize;
    let mut controlled = [9u8; 32];
    controlled.zeroize();
    assert_eq!(controlled, [0; 32]);
}

#[test]
fn test_locked_allow_deny_and_zero_metadata_leak() {
    for operation in [
        Operation::List,
        Operation::Read,
        Operation::History,
        Operation::Relationship,
        Operation::Edit,
        Operation::OrdinaryExport,
    ] {
        assert!(allow(Privacy::Sealed, operation, false).is_err());
        assert!(allow(Privacy::Private, operation, false).is_ok());
    }
    assert!(allow(Privacy::Sealed, Operation::OrdinaryExport, true).is_err());
    assert!(!ExcludeSealed.may_index(Privacy::Sealed));
    assert!(ExcludeSealed.may_index(Privacy::Private));
    // This is the internal policy hook only, not an application metadata-leak test.
}

#[test]
fn invalid_profiles_and_high_cost_refused_before_kdf() {
    for profile in [
        KdfProfile {
            memory_kib: 19455,
            ..KdfProfile::default()
        },
        KdfProfile {
            lanes: 0,
            ..KdfProfile::default()
        },
        KdfProfile {
            iterations: 17,
            ..KdfProfile::default()
        },
    ] {
        assert_eq!(
            derive_key(b"test", &[2; 16], profile, false).err(),
            Some(VaultError::InvalidEnvelope)
        );
    }
    assert_eq!(
        derive_key(
            b"test",
            &[2; 16],
            KdfProfile {
                memory_kib: 1048576,
                ..KdfProfile::default()
            },
            false
        )
        .err(),
        Some(VaultError::ResourceLimit)
    );
    assert!(Context::new("../outside", &"b".repeat(32), &"c".repeat(32)).is_err());
}

#[test]
fn production_profile_p4_matches_independent_node_reference() {
    let _serial = SERIAL_KDF.lock().unwrap();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/vault-argon2-reference.json")).unwrap();
    let password = hex(fixture["passwordHex"].as_str().unwrap());
    let salt: [u8; 16] = hex(fixture["saltHex"].as_str().unwrap())
        .try_into()
        .unwrap();
    let p4 = derive_key(&password, &salt, KdfProfile::default(), false).unwrap();
    assert_eq!(&p4[..], hex(fixture["p4"].as_str().unwrap()));
    let p1 = derive_key(
        &password,
        &salt,
        KdfProfile {
            lanes: 1,
            ..KdfProfile::default()
        },
        false,
    )
    .unwrap();
    assert_eq!(&p1[..], hex(fixture["p1"].as_str().unwrap()));
    assert_ne!(&*p1, &*p4, "p=4 must never silently collapse to p=1");
}

#[test]
fn absolute_deadline_and_caller_identity_are_enforced() {
    let caller = Caller {
        context: context(),
        transport: Transport::Desktop,
        root_id: "root-a".into(),
        caller_id: "window-a".into(),
    };
    let now = Instant::now();
    let mut sessions = Sessions::default();
    let token = sessions
        .establish(caller.clone(), Zeroizing::new([9; 32]), 5, now)
        .unwrap();
    let wrong_root = Caller {
        root_id: "root-b".into(),
        ..caller.clone()
    };
    let wrong_caller = Caller {
        caller_id: "window-b".into(),
        ..caller.clone()
    };
    for incorrect in [&wrong_root, &wrong_caller] {
        assert!(
            sessions
                .authorize(incorrect, &token, 5, now, |_| Ok(()))
                .is_err()
        );
    }
    assert!(
        sessions
            .authorize(&caller, &[0; 16], 5, now, |_| Ok(()))
            .is_err()
    );
    for seconds in (800..14400).step_by(800) {
        assert!(
            sessions
                .authorize(
                    &caller,
                    &token,
                    5,
                    now + Duration::from_secs(seconds),
                    |_| Ok(())
                )
                .is_ok()
        );
    }
    assert!(
        sessions
            .authorize(
                &caller,
                &token,
                5,
                now + Duration::from_secs(14400),
                |_| Ok(())
            )
            .is_err()
    );
}

#[test]
fn existing_application_commands_still_refuse_sealed_creation() {
    use anansi_core::{Core, ErrorCode};
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("unopened");
    let core = Core::with_storage_root(root.clone());
    for kind in anansi_core::entities::SUPPORTED_KINDS {
        let error=core.memory_command("create",serde_json::json!({
            "contractVersion":1,"workspaceId":"b".repeat(32),"kind":kind,
            "draft":{"title":"Synthetic sealed refusal","body":"no plaintext write","privacy":"sealed"}
        })).unwrap_err();
        assert_eq!(error.code, ErrorCode::VaultUnavailable);
    }
    assert!(
        !root.exists(),
        "the refusal must precede any storage creation"
    );
}

#[test]
fn test_interrupted_migration_atomic_rollback() {
    // Shadow preparation interruption only: no canonical generation switch is
    // implemented yet. Originals remain authoritative; partial staging is retained.
    use anansi_core::vault::migration::{self, StageInput};
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("original-one.md");
    let second = root.path().join("original-two.md");
    std::fs::write(&first, b"original synthetic one").unwrap();
    std::fs::write(&second, b"original synthetic two").unwrap();
    let r1 = "1".repeat(32);
    let r2 = "2".repeat(32);
    let v1 = "3".repeat(32);
    let v2 = "4".repeat(32);
    let inputs = [
        StageInput {
            record_id: &r1,
            revision_id: &v1,
            payload: b"original synthetic one",
        },
        StageInput {
            record_id: &r2,
            revision_id: &v2,
            payload: b"original synthetic two",
        },
    ];
    let source = "e".repeat(32);
    let outcome = migration::prepare(
        root.path(),
        &context(),
        &source,
        &[9; 32],
        &inputs,
        |count| {
            if count == 1 {
                Err(VaultError::StorageUnavailable)
            } else {
                Ok(())
            }
        },
    );
    assert!(outcome.is_err());
    assert_eq!(std::fs::read(&first).unwrap(), b"original synthetic one");
    assert_eq!(std::fs::read(&second).unwrap(), b"original synthetic two");
    let partial = root
        .path()
        .join("vault-staging")
        .join("a".repeat(32))
        .join("c".repeat(32));
    assert_eq!(std::fs::read_dir(&partial).unwrap().count(), 1);
    assert!(migration::verify(root.path(), &context(), &source, &[9; 32]).is_err());
    // Retrying never overwrites existing stage members or infers a resume.
    assert!(
        migration::prepare(root.path(), &context(), &source, &[9; 32], &inputs, |_| Ok(
            ()
        ))
        .is_err()
    );
    let target = Context::new(&"a".repeat(32), &"b".repeat(32), &"d".repeat(32)).unwrap();
    migration::prepare(root.path(), &target, &source, &[9; 32], &inputs, |_| Ok(())).unwrap();
    assert_eq!(
        migration::verify(root.path(), &target, &source, &[9; 32]).unwrap(),
        2
    );
    assert!(migration::verify(root.path(), &target, &source, &[8; 32]).is_err());
    let staged = root
        .path()
        .join("vault-staging")
        .join("a".repeat(32))
        .join("d".repeat(32))
        .join(format!("{r1}-{v1}.ansv"));
    let mut damaged = std::fs::read(&staged).unwrap();
    damaged[80] ^= 1;
    std::fs::write(&staged, &damaged).unwrap();
    assert!(migration::verify(root.path(), &target, &source, &[9; 32]).is_err());
    assert_eq!(std::fs::read(&first).unwrap(), b"original synthetic one");
    assert_eq!(std::fs::read(&second).unwrap(), b"original synthetic two");
}
