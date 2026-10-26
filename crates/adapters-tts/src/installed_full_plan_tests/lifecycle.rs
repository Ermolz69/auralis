use super::*;
use ports::PortError;

pub fn unconfirmed(message: impl Into<String>) -> PortError {
    PortError::CleanupFailed {
        resource: "installed metadata worker".into(),
        message: message.into(),
    }
}

pub fn uncertain(outcome: &TestResult<Value>) -> bool {
    let mut error = outcome.as_ref().err().map(|error| error.as_ref());
    while let Some(current) = error {
        if matches!(
            current.downcast_ref::<PortError>(),
            Some(PortError::CleanupFailed { .. })
        ) {
            return true;
        }
        error = current.source();
    }
    false
}

pub fn release_candidate<T>(lease: T, outcome: &TestResult<Value>) -> Option<T> {
    if uncertain(outcome) {
        std::mem::forget(lease);
        None
    } else {
        Some(lease)
    }
}

pub fn preserve<T>(operation: TestResult<T>, uncertainty: bool) -> TestResult<T> {
    operation.map_err(|error| {
        if uncertainty {
            unconfirmed(format!("uncertain cleanup; receipt error: {error}")).into()
        } else {
            error
        }
    })
}

#[test]
fn unconfirmed_exit_retains_the_lease_without_native_admission() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct FakeLease(Arc<AtomicUsize>);
    impl Drop for FakeLease {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let outcome: TestResult<Value> = Err(unconfirmed("inert exit query failed").into());
    let returned = release_candidate(FakeLease(drops.clone()), &outcome);
    assert!(
        returned.is_none(),
        "unconfirmed native release was permitted"
    );
    drop(returned);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    for outcome in [
        Ok(json!({})),
        Err(PortError::InvalidSource {
            message: "known validation failure after confirmed exit".into(),
        }
        .into()),
    ] {
        let returned = release_candidate(FakeLease(drops.clone()), &outcome);
        assert!(returned.is_some());
        drop(returned);
    }
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    let missing_identity = super::proof::confirm_report(&json!({})).map(|()| json!({}));
    assert!(
        uncertain(&missing_identity),
        "independent identity failure lost its cleanup type"
    );
    let receipt: TestResult<Value> = preserve(
        Err(std::io::Error::other("inert receipt failure").into()),
        true,
    );
    assert!(
        uncertain(&receipt),
        "receipt error erased cleanup uncertainty"
    );
}
