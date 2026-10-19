use anansi_core::{
    privacy::Privacy,
    vault::{
        VaultError,
        keys::Context,
        policy::SearchBoundary,
        session::{Caller, Sessions, Transport},
    },
};
use std::time::Instant;
use zeroize::Zeroizing;

fn caller() -> Caller {
    Caller {
        context: Context::new(&"a".repeat(32), &"b".repeat(32), &"c".repeat(32)).unwrap(),
        transport: Transport::Desktop,
        root_id: "root".into(),
        caller_id: "window".into(),
    }
}

#[test]
fn non_sealed_search_publishes_without_vault_session() {
    let mut sessions = Sessions::default();
    let result = SearchBoundary::publish(
        &mut sessions,
        &caller(),
        &[0; 16],
        1,
        Instant::now(),
        Privacy::Private,
        || Ok::<_, VaultError>("visible"),
    );
    assert_eq!(result.unwrap(), "visible");
}

#[test]
fn sealed_search_requires_current_bound_session_and_epoch() {
    let c = caller();
    let now = Instant::now();
    let mut sessions = Sessions::default();
    let token = sessions
        .establish(c.clone(), Zeroizing::new([7; 32]), 4, now)
        .unwrap();
    let mut calls = 0;
    let visible =
        SearchBoundary::publish(&mut sessions, &c, &token, 4, now, Privacy::Sealed, || {
            calls += 1;
            Ok::<_, VaultError>("sealed result")
        })
        .unwrap();
    assert_eq!(visible, "sealed result");
    assert_eq!(calls, 1);
    assert_eq!(
        SearchBoundary::publish(&mut sessions, &c, &token, 5, now, Privacy::Sealed, || Ok::<
            _,
            VaultError,
        >(
            "must not publish"
        ))
        .unwrap_err(),
        VaultError::Locked
    );
    assert_eq!(
        SearchBoundary::publish(&mut sessions, &c, &token, 4, now, Privacy::Sealed, || Ok::<
            _,
            VaultError,
        >(
            "must not publish"
        ))
        .unwrap_err(),
        VaultError::Locked
    );
}

#[test]
fn sealed_records_are_never_indexable() {
    assert!(!SearchBoundary::may_index(Privacy::Sealed));
    assert!(SearchBoundary::may_index(Privacy::Private));
    assert!(SearchBoundary::may_index(Privacy::Standard));
    assert!(SearchBoundary::may_index(Privacy::Public));
}
