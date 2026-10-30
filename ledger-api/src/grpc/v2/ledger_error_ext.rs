/// Extension of a decoded canton error, providing Ledger API specific methods.
pub trait LedgerErrorExt {
    /// Completion offset extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the completion offset
    /// of the succeeded command.
    ///
    /// Note: this will also return `None`, if completion offset failed to be parsed as `i64`
    fn completion_offset(&self) -> Option<i64>;

    /// Existing submission ID extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the submission ID of the
    /// succeeded command.
    ///
    /// Note: this will also return `None`, if submission ID failed to be parsed as `LedgerString`
    fn existing_submission_id(&self) -> Option<LedgerString>;

    /// Application-defined error ID inside `DAML_FAILURE`
    fn daml_error_id(&self) -> Option<&str>;
}

impl LedgerErrorExt for canton_base::error::DecodedCantonError {
    fn completion_offset(&self) -> Option<i64> {
        self.metadata()
            .get("completion_offset")
            .and_then(|offset| offset.parse::<i64>().ok())
    }

    fn existing_submission_id(&self) -> Option<LedgerString> {
        self.metadata()
            .get("existing_submission_id")
            .cloned()
            .map(LedgerString::new)
            .transpose()
            .ok()
            .flatten()
    }

    fn daml_error_id(&self) -> Option<&str> {
        self.metadata().get("error_id").map(String::as_str)
    }
}
