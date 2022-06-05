use bindgen::callbacks::EnumVariantValue;
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct ParseCallbacks;

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        original_variant_name
            .strip_prefix("XWII_")
            .map(|str| str.to_string())
    }

    fn item_name(&self, original_item_name: &str) -> Option<String> {
        original_item_name
            .to_ascii_lowercase()
            .strip_prefix("xwii_")
            .map(|str| str.to_string())
    }

    fn include_file(&self, filename: &str) {
        // Invalidate the built crate whenever any of the included
        // header files changed (copied from `bindgen::CargoCallbacks`).
        println!("cargo:rerun-if-changed={}", filename);
    }
}

fn main() {
    // Invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .allowlist_type("xwii_.*")
        .allowlist_function("xwii_.*")
        .allowlist_var("XWII_.*")
        .size_t_is_usize(true)
        .prepend_enum_name(false)
        .parse_callbacks(Box::new(ParseCallbacks {}))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
