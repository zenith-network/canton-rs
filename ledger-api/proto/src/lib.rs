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

                    tonic::include_proto!("com.daml.ledger.api.v2");

                    #[cfg(feature = "v2-admin")]
                    pub mod admin {
                        tonic::include_proto!("com.daml.ledger.api.v2.admin");
                    }
                    #[cfg(feature = "v2-interactive")]
                    pub mod interactive {
                        tonic::include_proto!("com.daml.ledger.api.v2.interactive");

                        #[cfg(feature = "v2-transaction")]
                        pub mod transaction {
                            #[cfg(feature = "v2-transaction-v1")]
                            pub mod v1 {
                                tonic::include_proto!(
                                    "com.daml.ledger.api.v2.interactive.transaction.v1"
                                );
                            }
                        }
                    }
                    #[cfg(feature = "v2-testing")]
                    pub mod testing {
                        tonic::include_proto!("com.daml.ledger.api.v2.testing");
                    }
                }
            }
        }
    }
}
// This feature gate expresses the fact that google.rpc is generated only as a dependency of the
// main protobuf set above. We don't need it to be here all the time.
#[cfg(feature = "v2")]
pub mod google {
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}

pub use prost;
