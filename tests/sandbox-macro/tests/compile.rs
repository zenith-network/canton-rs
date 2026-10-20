#[test]
fn sandbox_attribute() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/pass/*.rs");
    tests.compile_fail("tests/fail/*.rs");
}
