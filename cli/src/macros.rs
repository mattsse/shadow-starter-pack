#[macro_export]
macro_rules! test_fixture {
    ($cname:expr, $fname:expr) => {
        format!(
            "{}/src/{}/fixtures/{}",
            env!("CARGO_MANIFEST_DIR"),
            $cname,
            $fname
        )
    };
}
