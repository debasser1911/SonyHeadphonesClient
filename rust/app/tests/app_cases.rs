#[test]
fn recorded_cases() {
    trycmd::TestCases::new()
        .register_bin(
            "sony-headphones-client",
            std::path::Path::new(env!("CARGO_BIN_EXE_sony-headphones-client")),
        )
        .case("../app-cases/*.toml");
}
