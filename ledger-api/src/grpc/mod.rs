// TODO: Remove this attribute when CantonError size issue is solved.
//       For now it's just easier to apply it here once for the whole module, than targeting every
//       affected function.
#[allow(clippy::result_large_err)]
#[cfg(feature = "v2")]
pub mod v2;
