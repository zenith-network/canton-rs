use crate::Name;

/// Marker trait which marks a type which represents a Daml template
pub trait Template {}

/// Type which represents a Daml template with a contract key
pub trait TemplateWithKey: Template {
    /// Contract key type
    type Key;
}

/// Type which represents a Daml choice
///
/// Generic type parametes defines template type. A single type may be a choice of many templates.
pub trait Choice<T: Template> {
    /// Whether this choice is consuming or not
    const CONSUMING: bool;

    /// Name of the choice
    const NAME: Name;

    /// Result type of the choice
    type Result;
}
