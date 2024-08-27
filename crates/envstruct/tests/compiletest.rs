#[cfg_attr(miri, ignore = "incompatible with miri")]
#[allow(unused_attributes)]
#[test]
fn compiletest() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compiletest/**/*.rs");
}
