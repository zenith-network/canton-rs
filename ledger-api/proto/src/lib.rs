#![cfg_attr(not(feature = "std"), no_std)]

// We want to keep the protobuf code aligned with the upstream, so we don't want to change it to fix
// these issues. That's why we simply silence the warnings.
#[allow(
    clippy::large_enum_variant,
    clippy::doc_overindented_list_items,
    clippy::doc_lazy_continuation,
    reason = "Generated code"
)]
pub mod com {
    pub mod daml {
        pub mod ledger {
            pub mod api {
                #[cfg(feature = "v2")]
                pub mod v2 {
                    pub use ledger_api_value_proto::com::daml::ledger::api::v2::*;

                    include!(concat!(env!("OUT_DIR"), "/com.daml.ledger.api.v2.rs"));

                    pub mod admin {
                        include!(concat!(env!("OUT_DIR"), "/com.daml.ledger.api.v2.admin.rs"));
                    }
                    pub mod interactive {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.daml.ledger.api.v2.interactive.rs"
                        ));

                        pub mod transaction {
                            pub mod v1 {
                                include!(concat!(
                                    env!("OUT_DIR"),
                                    "/com.daml.ledger.api.v2.interactive.transaction.v1.rs"
                                ));
                            }
                        }
                    }
                    #[cfg(feature = "v2-testing")]
                    pub mod testing {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.daml.ledger.api.v2.testing.rs"
                        ));
                    }
                }
            }
        }
    }
}
// This feature gate expresses the fact that google.rpc is generated only as a dependency of the
// main protobuf set above. We don't need it to be here all the time.
#[cfg(feature = "v2")]
// Some of the links inside generated code are broken. We can't do anything about it, cause we don't
// want to modify vendored protobuf code, so we just silence the warnings.
#[allow(rustdoc::broken_intra_doc_links)]
pub mod google {
    pub mod rpc {
        include!(concat!(env!("OUT_DIR"), "/google.rpc.rs"));
    }
}

// Re-export
pub use prost;
pub use prost_types;
#[cfg(any(feature = "client", feature = "server"))]
pub use tonic;
