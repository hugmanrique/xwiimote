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

#[cfg(target_os = "linux")]
fn main() {
    println!("cargo:rustc-link-lib=udev");

    // Invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=xwiimote/lib/core.c");
    println!("cargo:rerun-if-changed=xwiimote/lib/monitor.c");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .allowlist_type("xwii_.*")
        .allowlist_function("xwii_.*")
        .allowlist_var("XWII_.*")
        .size_t_is_usize(true)
        .derive_default(true)
        .prepend_enum_name(false)
        .parse_callbacks(Box::new(ParseCallbacks {}))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    cc::Build::new()
        .file("xwiimote/lib/core.c")
        .file("xwiimote/lib/monitor.c")
        // The non-used enum-array entries are initialized to -1 using
        // the designated initializer [0 ... MAX] = -1, which causes a
        // double initialization when the entry of each enum variant is
        // initialized. This is mostly harmless, so we ignore it.
        .flag("-Wno-override-init")
        .define("XWII__EXPORT", r#"__attribute__((visibility("default")))"#)
        .compile("xwiimote");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("xwiimote only works on Linux");
}
