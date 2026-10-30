//! Error handling

pub mod category_id;
pub mod decoded_canton_error;
pub mod error_code_id;
pub mod redacted_canton_error;

pub use category_id::CategoryId;
pub use decoded_canton_error::DecodedCantonError;
pub use error_code_id::ErrorCodeId;
pub use redacted_canton_error::RedactedCantonError;
