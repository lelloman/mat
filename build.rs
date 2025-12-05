use std::env;
use std::path::Path;

fn main() {
    // Rerun if syntax files change
    println!("cargo:rerun-if-changed=assets/syntaxes/");

    let out_dir = env::var("OUT_DIR").unwrap();
    let syntax_set_path = Path::new(&out_dir).join("syntax_set.packdump");

    // Build syntax set with defaults + custom syntaxes
    let mut builder = syntect::parsing::SyntaxSet::load_defaults_newlines().into_builder();

    // Add custom syntaxes from assets/syntaxes/
    let syntaxes_dir = Path::new("assets/syntaxes");
    if syntaxes_dir.exists() {
        builder
            .add_from_folder(syntaxes_dir, true)
            .expect("Failed to load custom syntaxes");
    }

    let syntax_set = builder.build();

    // Serialize to packdump file
    syntect::dumps::dump_to_uncompressed_file(&syntax_set, &syntax_set_path)
        .expect("Failed to write syntax set");
}
