//! Daml `Time` and `Date`, as the Ledger API encodes them.

/// Daml `Time`: microseconds since the Unix epoch (UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

/// Daml `Date`: days since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i32);
