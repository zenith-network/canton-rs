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
pub mod google {
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}

pub use prost;
