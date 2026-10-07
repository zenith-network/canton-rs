// We want to keep the protobuf code aligned with the upstream, so we don't want to change it to fix
// these issues. That's why we simply silence the warnings.
#[allow(
    clippy::large_enum_variant,
    clippy::doc_overindented_list_items,
    clippy::doc_lazy_continuation,
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_html_tags,
    reason = "Generated code"
)]
pub mod com {
    pub mod digitalasset {
        pub mod canton {
            // Re-export
            pub use admin_api_proto::com::digitalasset::canton::admin;

            pub mod connection {
                #[cfg(feature = "v30")]
                pub mod v30 {
                    tonic::include_proto!("com.digitalasset.canton.connection.v30");
                }
            }
            pub mod crypto {
                pub mod admin {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.crypto.admin.v30");
                    }
                }
                #[cfg(feature = "v30")]
                pub mod v30 {
                    tonic::include_proto!("com.digitalasset.canton.crypto.v30");
                }
                #[cfg(feature = "v31")]
                pub mod v31 {
                    tonic::include_proto!("com.digitalasset.canton.crypto.v31");
                }
            }
            pub mod protocol {
                #[cfg(feature = "v30")]
                pub mod v30 {
                    tonic::include_proto!("com.digitalasset.canton.protocol.v30");
                }
                #[cfg(feature = "v31")]
                pub mod v31 {
                    tonic::include_proto!("com.digitalasset.canton.protocol.v31");
                }
                #[cfg(feature = "v32")]
                pub mod v32 {
                    tonic::include_proto!("com.digitalasset.canton.protocol.v32");
                }
            }
            pub mod sequencer {
                pub mod api {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.sequencer.api.v30");
                    }
                }
            }
            pub mod synchronizer {
                #[cfg(feature = "v30")]
                pub mod v30 {
                    tonic::include_proto!("com.digitalasset.canton.synchronizer.v30");
                }
            }
            pub mod time {
                pub mod admin {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        tonic::include_proto!("com.digitalasset.canton.time.admin.v30");
                    }
                }
            }
            pub mod topology {
                pub mod admin {
                    #[cfg(feature = "v30")]
                    pub mod v30 {
                        // Re-export
                        pub use admin_api_proto::com::digitalasset::canton::topology::admin::v30::*;

                        tonic::include_proto!("com.digitalasset.canton.topology.admin.v30");
                    }
                }
            }
            #[cfg(feature = "v30")]
            pub mod v30 {
                tonic::include_proto!("com.digitalasset.canton.v30");
            }
            pub mod version {
                pub mod v1 {
                    tonic::include_proto!("com.digitalasset.canton.version.v1");
                }
            }
        }
    }
}

#[cfg(feature = "v30")]
// Some of the links inside generated code are broken. We can't do anything about it, cause we don't
// want to modify vendored protobuf code, so we just silence the warnings.
#[allow(rustdoc::broken_intra_doc_links)]
pub mod google {
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}
