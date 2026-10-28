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
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.crypto.v30.rs"
                        ));
                    }
                }
                pub mod health {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.health.v30.rs"
                        ));
                    }
                }
                pub mod mediator {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.mediator.v30.rs"
                        ));
                    }
                }
                pub mod participant {
                    // Some docs are treated as a doc tests, however they are not supposed to be
                    // them. So we disable doctests for this generated module. No doc tests will
                    // be collected from it.
                    #[cfg(not(doctest))]
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.participant.v30.rs"
                        ));
                    }
                }
                pub mod pruning {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.pruning.v30.rs"
                        ));
                    }
                }
                pub mod sequencer {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.sequencer.v30.rs"
                        ));
                    }
                }
                pub mod time {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.time.v30.rs"
                        ));
                    }
                }
                pub mod topology {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.admin.topology.v30.rs"
                        ));
                    }
                }
            }
            pub mod topology {
                pub mod admin {
                    pub mod v30 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/com.digitalasset.canton.topology.admin.v30.rs"
                        ));
                    }
                }
            }
        }
    }
}
