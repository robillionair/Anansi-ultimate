//! Explicit opt-in synthetic OS-store verification; never run on real vault IDs.
#[cfg(windows)]
#[test]
#[ignore = "writes and removes one unique synthetic Windows credential; run explicitly"]
fn native_wrapped_slot_roundtrip() {
    use anansi_core::vault::{credential_store::*, envelope, keys::*};
    let id: String = envelope::random::<16>()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let context = Context::new(&id, &"b".repeat(32), &"c".repeat(32)).unwrap();
    let (slots, _recovery) =
        KeySlots::initialize(&context, b"synthetic OS test passphrase").unwrap();
    let token =
        WrappedSlot::authenticate(&context, &slots.passphrase, b"synthetic OS test passphrase")
            .unwrap();
    assert!(save(&context, &token, false).is_err());
    // Cleanup runs after any post-write assertion failure and targets only this ID.
    struct Cleanup(Context);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = forget(&self.0, true);
        }
    }
    let cleanup = Cleanup(context.clone());
    save(&context, &token, true).unwrap();
    let loaded = load(&context).unwrap();
    assert_eq!(&*loaded, token.bytes());
    assert!(WrappedSlot::authenticate(&context, &loaded, b"wrong").is_err());
    let swapped = Context::new(&id, &"d".repeat(32), &"c".repeat(32)).unwrap();
    assert!(save(&swapped, &token, true).is_err());
    forget(&context, true).unwrap();
    assert!(load(&context).is_err());
    drop(cleanup);
}
