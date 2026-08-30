use trybuild::TestCases;

#[test]
fn ui_compile_fail() {
    let t = TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
