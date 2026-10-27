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
            pub mod connection {
                pub mod v30 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.connection.v30.rs"
                    ));
                }
            }
            pub mod crypto {
                pub mod admin {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.crypto.admin.v30.rs"
                        ));
                    }
                }
                pub mod v30 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.crypto.v30.rs"
                    ));
                }
            }
            pub mod protocol {
                pub mod v30 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.protocol.v30.rs"
                    ));
                }
                pub mod v31 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.protocol.v31.rs"
                    ));
                }
                pub mod v32 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.protocol.v32.rs"
                    ));
                }
            }
            pub mod sequencer {
                pub mod api {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.sequencer.api.v30.rs"
                        ));
                    }
                }
            }
            pub mod synchronizer {
                pub mod v30 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.synchronizer.v30.rs"
                    ));
                }
            }
            pub mod time {
                pub mod admin {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.time.admin.v30.rs"
                        ));
                    }
                }
            }
            pub mod topology {
                pub mod admin {
                    pub mod v30 {
                        // Re-export here for convenience
                        pub use admin_api_proto::com::digitalasset::canton::topology::admin::v30::*;

                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.topology.admin.v30.rs"
                        ));
                    }
                }
            }
            pub mod v30 {
                include!(concat!(env!("OUT_DIR"), "/com.digitalasset.canton.v30.rs"));
            }
            pub mod version {
                pub mod v1 {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/com.digitalasset.canton.version.v1.rs"
                    ));
                }
            }
        }
    }
}

// Some of the links inside generated code are broken. We can't do anything about it, cause we don't
// want to modify vendored protobuf code, so we just silence the warnings.
#[allow(rustdoc::broken_intra_doc_links)]
pub mod google {
    pub mod rpc {
        include!(concat!(env!("OUT_DIR"), "/google.rpc.rs"));
    }
}

// Re-export
pub use admin_api_proto;
