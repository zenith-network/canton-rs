// We want to keep the protobuf code aligned with the upstream, so we don't want to change it to fix
// these issues. That's why we simply silence the warnings.
#[allow(
    clippy::large_enum_variant,
    clippy::doc_overindented_list_items,
    clippy::doc_lazy_continuation,
    rustdoc::invalid_html_tags,
    reason = "Generated code"
)]
pub mod com {
    pub mod digitalasset {
        pub mod canton {
            pub mod admin {
                pub mod crypto {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.crypto.v30");
                    }
                }
                pub mod health {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.health.v30");
                    }
                }
                pub mod mediator {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.mediator.v30");
                    }
                }
                pub mod participant {
                    #[cfg(feature = "v30")]
                    // Some docs are treated as a doc tests, however they are not supposed to be
                    // them. So we disable doctests for this generated module. No doc tests will
                    // be collected from it.
                    #[cfg(not(doctest))]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.participant.v30");
                    }
                }
                pub mod pruning {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.pruning.v30");
                    }
                }
                pub mod sequencer {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.sequencer.v30");
                    }
                }
                pub mod time {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.time.v30");
                    }
                }
                pub mod topology {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.admin.topology.v30");
                    }
                }
            }
            pub mod topology {
                pub mod admin {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.topology.admin.v30");
                    }
                }
            }
        }
    }
}
