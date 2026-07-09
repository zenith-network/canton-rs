pub mod com {
    pub mod daml {
        pub mod ledger {
            pub mod api {
                #[cfg(feature = "v2")]
                pub mod v2 {
                    include!(concat!(env!("OUT_DIR"), "/com.daml.ledger.api.v2.rs"));
                }
            }
        }
    }
}
