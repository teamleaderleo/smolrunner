use smolrunner::state::StatePath;
use smolrunner::state_store::{
    StateRead, StateRecord, StateStore, StateStoreError, StateStoreErrorKind,
    StateWriteDisposition, StateWriteReceipt,
};

struct ExternalStore;

impl StateStore for ExternalStore {
    fn read(&self, _path: &StatePath) -> Result<StateRead, StateStoreError> {
        Err(StateStoreError::public(
            StateStoreErrorKind::Busy,
            "state store is busy",
        ))
    }

    fn write_atomic(
        &mut self,
        record: &StateRecord,
    ) -> Result<StateWriteReceipt, StateStoreError> {
        Ok(StateWriteReceipt::new(
            StateWriteDisposition::Created,
            record.bytes().len(),
        ))
    }
}

#[test]
fn external_store_implementations_can_return_bounded_public_results() {
    let store = ExternalStore;
    let error = store
        .read(unsafe_unreachable_state_path())
        .expect_err("test store reports a bounded busy error");

    assert_eq!(error.kind(), StateStoreErrorKind::Busy);
    assert_eq!(error.message(), "state store is busy");

    let receipt = StateWriteReceipt::new(StateWriteDisposition::Created, 42);
    assert_eq!(receipt.disposition(), StateWriteDisposition::Created);
    assert_eq!(receipt.bytes_written(), 42);
}

fn unsafe_unreachable_state_path() -> &'static StatePath {
    panic!("the rejecting test store never inspects its path argument")
}
