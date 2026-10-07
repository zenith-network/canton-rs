//! Compare decoded Canton errors with independently observed server details.

use std::collections::HashMap;

use ledger_api::grpc::v2::error::{CantonError, DecodedCantonError};
use pretty_assertions::assert_eq;
use tonic_types::StatusExt;

use super::strings;

/// Require a structured Canton error, retaining other variants in failure diagnostics.
pub fn expect_decoded(error: CantonError) -> DecodedCantonError {
    match error {
        CantonError::Decoded(error) => error,
        other => panic!("expected enriched error, got {other:#?}"),
    }
}

/// Compare error codes, categories, metadata, resources, and request details.
pub fn assert_error(error: &DecodedCantonError, raw: &tonic::Status, reason: &str) {
    let info = raw.get_details_error_info().expect("real server ErrorInfo");
    assert_eq!(info.reason, reason);
    assert_eq!(error.error_code_id().as_str(), reason);
    assert_eq!(error.code(), raw.code());
    assert_eq!(
        i32::from(error.category_id()),
        info.metadata["category"].parse::<i32>().unwrap()
    );
    assert_eq!(
        error.definite_answer(),
        info.metadata
            .get("definite_answer")
            .map(|v| v.parse().unwrap())
    );
    assert_metadata(error, info.metadata);
    assert_resources(error, raw);
    assert_eq!(
        error.correlation_id().is_some(),
        raw.get_details_request_info().is_some()
    );
    assert_eq!(
        error.trace_id().is_some(),
        raw.get_details_error_info()
            .unwrap()
            .metadata
            .contains_key("tid")
    );
    assert_eq!(
        error.retry_delay(),
        raw.get_details_retry_info()
            .and_then(|info| info.retry_delay)
    );
    assert!(!error.full_message().is_empty());
    assert!(error.full_message().starts_with(reason));
    assert_eq!(
        error.message(),
        error
            .full_message()
            .split_once(':')
            .map(|(_, msg)| msg.trim())
    );
}

/// Account for request-specific command fields while comparing server metadata.
fn assert_metadata(error: &DecodedCantonError, metadata: HashMap<String, String>) {
    let metadata: HashMap<_, _> = metadata
        .into_iter()
        .filter(|(key, _)| !["category", "tid", "definite_answer"].contains(&key.as_str()))
        .collect();
    assert_eq!(strings(error.metadata().keys()), strings(metadata.keys()));
    for (key, expected) in &metadata {
        // Commands contain different submission IDs and server-assigned
        // submittedAt values because these are two independent RPCs.
        if key == "commands" {
            assert!(error.metadata()[key].contains("commandId:"));
            assert!(error.metadata()[key].contains("submittedAt:"));
        } else {
            assert_eq!(&error.metadata()[key], expected, "metadata field {key}");
        }
    }
}

/// Compare every reported resource with the protobuf error details.
fn assert_resources(error: &DecodedCantonError, raw: &tonic::Status) {
    let resources: Vec<_> = raw
        .get_error_details_vec()
        .into_iter()
        .filter_map(|detail| match detail {
            tonic_types::ErrorDetail::ResourceInfo(resource) => Some(resource),
            _ => None,
        })
        .collect();
    assert_eq!(error.resources().len(), resources.len());
    for (actual, expected) in error.resources().iter().zip(resources) {
        assert_eq!(actual.resource_type, expected.resource_type);
        assert_eq!(actual.resource_name, expected.resource_name);
        assert_eq!(actual.owner, expected.owner);
        assert_eq!(actual.description, expected.description);
    }
}
