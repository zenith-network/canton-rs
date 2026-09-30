use crate::Name;

/// A type which is either template or interface
pub trait TemplateOrInterface {}

/// Marker trait which marks a type which represents a Daml template
pub trait Template: TemplateOrInterface {}

/// Type which represents a Daml template with a contract key
pub trait TemplateWithKey: Template {
    /// Contract key type
    type Key;
}

/// Type which represents a Daml choice
///
/// Generic type parametes defines template type. A single type may be a choice of many templates.
pub trait Choice<R: TemplateOrInterface> {
    /// Whether this choice is consuming or not
    const CONSUMING: bool;

    /// Name of the choice
    const NAME: Name;

    /// Result type of the choice
    type Result;
}

/// Type which represents a Daml interface
pub trait Interface: TemplateOrInterface {
    type View;
}

/// A template which implements an interface
pub trait Implements<I: Interface>: Template {}

/// An interface which requires another interface
pub trait Requires<I: Interface>: Interface {}
