//! Error handling

mod canton_error;
mod category_id;
mod decoded_canton_error;
mod error_code_id;
mod redacted_canton_error;

#[cfg(test)]
mod test_fixtures;

pub use canton_error::CantonError;
pub use category_id::{CategoryId, UnknownCategoryId};
pub use decoded_canton_error::DecodedCantonError;
pub use error_code_id::ErrorCodeId;
pub use redacted_canton_error::RedactedCantonError;
