fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("slintplot".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config("ui/fuzz.slint", config).unwrap();
}
