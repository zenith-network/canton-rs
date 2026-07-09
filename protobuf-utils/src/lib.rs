use std::error::Error;

/// Error which occurs when a required protobuf field is missing
#[derive(Debug, thiserror::Error)]
#[error("required field '{pkg_name}.{msg_name}.{field_name}' is missing")]
pub struct MissingProtoField {
    pkg_name: String,
    msg_name: String,
    field_name: String,
}

impl MissingProtoField {
    pub fn new(
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Self {
        Self {
            pkg_name: pkg_name.into(),
            msg_name: msg_name.into(),
            field_name: field_name.into(),
        }
    }
}

/// Helper trait to deal with required protobuf fields
///
/// # Example
///
/// ```rust,no_run
/// # use protobuf_utils::RequiredProtoField;
/// fn func() -> u64 {
///     let x: Option<u64> = None;
///     let y = x.required("my.pkg", "MyMessage", "my_field").unwrap();
///     y
/// }
/// ```
pub trait RequiredProtoField {
    type Value;

    fn required(
        self,
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Result<Self::Value, MissingProtoField>;

    /// Required field with context defined by message type
    fn required_of<T: prost::Name>(
        self,
        field_name: impl Into<String>,
    ) -> Result<Self::Value, MissingProtoField>
    where
        Self: Sized,
    {
        self.required(T::PACKAGE, T::NAME, field_name)
    }
}

impl<T> RequiredProtoField for Option<T> {
    type Value = T;

    /// Return error if required field is `None`
    ///
    /// This is just a convenient wrapper for [`Option::ok_or_else()`].
    fn required(
        self,
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Result<Self::Value, MissingProtoField> {
        self.ok_or_else(|| MissingProtoField::new(pkg_name, msg_name, field_name))
    }
}

// TODO: I guess this can be reasonably implemented for Vec and String?

/// Error which occurs when a field failed to be validated
#[derive(Debug, thiserror::Error)]
#[error("value of the field '{pkg_name}.{msg_name}.{field_name}' is invalid")]
pub struct InvalidProtoFieldValue {
    pkg_name: String,
    msg_name: String,
    field_name: String,
    #[source]
    source: Box<dyn Error + 'static + Send + Sync>,
}

impl InvalidProtoFieldValue {
    pub fn new(
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
        source: impl Error + 'static + Send + Sync,
    ) -> Self {
        Self {
            pkg_name: pkg_name.into(),
            msg_name: msg_name.into(),
            field_name: field_name.into(),
            source: Box::new(source),
        }
    }
}

/// Helper trait to deal with validating protobuf fields
///
/// # Example
///
/// ```rust,no_run
/// # use protobuf_utils::InvalidProtoField;
/// fn func<E: std::error::Error>() -> u64 {
///     let x: Result<u64, E> = Ok(1);
///     let y = x.validated("my.pkg", "MyMessage", "my_field").unwrap();
///     y
/// }
/// ```
pub trait InvalidProtoField {
    type Ok;

    fn validated(
        self,
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Result<Self::Ok, InvalidProtoFieldValue>;

    /// Validated field with context defined by message type
    fn validated_of<T: prost::Name>(
        self,
        field_name: impl Into<String>,
    ) -> Result<Self::Ok, InvalidProtoFieldValue>
    where
        Self: Sized,
    {
        self.validated(T::PACKAGE, T::NAME, field_name)
    }
}

impl<T, E: Error + 'static + Send + Sync> InvalidProtoField for Result<T, E> {
    type Ok = T;

    /// Wrap error with protobuf field context
    ///
    /// This is just a convenient wrapper for [`Result::map_err`].
    fn validated(
        self,
        pkg_name: impl Into<String>,
        msg_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Result<T, InvalidProtoFieldValue> {
        self.map_err(|err| InvalidProtoFieldValue::new(pkg_name, msg_name, field_name, err))
    }
}
