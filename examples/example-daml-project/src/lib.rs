use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/main_package.rs"));

use crate::package_c3bb0c5d04799b3f11bad7c3c102963e115cf53da3e4afcbcfd9f06ebd82b4ff::da::set::types::Set;
use crate::my_contracts::MyTemplate;

use canton::ledger_api::grpc::v2::client::CantonClient;
use canton::types::PartyId;

pub fn test_something() {
    let client = CantonClient::builder("http://localhost")
        .connect_lazy()
        .unwrap();
    // client.command().submit_and_wait(commands);

    let temp = MyTemplate {
        values: Set {
            map: BTreeMap::from([(1, ())]),
        },
        owner: PartyId::new("someid::somens".to_string()).unwrap(),
    };
    temp.create();
}
