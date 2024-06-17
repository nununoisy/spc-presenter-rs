use std::path::Path;
use slint_build;

fn compile<P: AsRef<Path>>(path: P) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![
            manifest_dir.join("assets")
        ])
        .with_style("fluent-dark".to_string());
    slint_build::compile_with_config(path, config).unwrap();
}

fn main() {
    compile("src/gui/slint/main.slint");
}
