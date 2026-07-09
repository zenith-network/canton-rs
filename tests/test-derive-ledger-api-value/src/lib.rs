use daml_lf_ledger_api_value::LedgerApiValue;

#[derive(LedgerApiValue)]
#[ledger_api_value(package_id = "ff", module_name = "A.B.C")]
pub struct MyType {
    value: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_my_type() {
        test_ledger_api_value(MyType { value: 1 });
    }

    pub fn test_ledger_api_value<V: LedgerApiValue>(value: V) {
        let proto = value.into_proto_value();
        dbg!(proto);
    }
}
