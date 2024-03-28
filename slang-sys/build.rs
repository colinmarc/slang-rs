extern crate bindgen;

use std::env;
use std::path::PathBuf;

use anyhow::{bail, Context};

fn main() -> anyhow::Result<()> {
    let out_dir = env::var("OUT_DIR")
        .map(PathBuf::from)
        .context("Couldn't determine output directory.")?;

    let slang_dir = env::var("SLANG_DIR").map(PathBuf::from);

    let (slang_h, slang_lib) = if let Ok(slang_dir) = slang_dir {
        let slang_h = slang_dir.join("include").join("slang.h");
        let slang_lib = locate_bin_dir(&slang_dir)?;
        (slang_h, slang_lib)
    } else {
        #[cfg(feature = "from-source")]
        {
            let dst = build_from_source();
            (dst.join("include").join("slang.h"), dst.join("lib"))
        }

        #[cfg(not(feature = "from-source"))]
        bail!(
            "Environment variable `SLANG_DIR` should be set to the directory of a Slang installation. \
            This directory should contain `slang.h` and a `bin` subdirectory.");
    };

    println!("cargo:rustc-link-search=native={}", slang_lib.display());
    println!("cargo:rustc-link-lib=dylib=slang");

    bindgen::builder()
        .header(slang_h.to_str().unwrap())
        .clang_arg("-v")
        .clang_arg("-xc++")
        .clang_arg("-std=c++14")
        .allowlist_function("slang_.*")
        .allowlist_type("slang.*")
        .allowlist_var("SLANG_.*")
        .with_codegen_config(
            bindgen::CodegenConfig::FUNCTIONS
                | bindgen::CodegenConfig::TYPES
                | bindgen::CodegenConfig::VARS,
        )
        .parse_callbacks(Box::new(ParseCallback {}))
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .vtable_generation(true)
        .layout_tests(false)
        .derive_copy(true)
        .generate()
        .context("Couldn't generate bindings.")?
        .write_to_file(out_dir.join("bindings.rs"))
        .context("Couldn't write bindings.")?;

    Ok(())
}

fn locate_bin_dir(slang_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Couldn't determine target OS.");

    let target_arch =
        env::var("CARGO_CFG_TARGET_ARCH").expect("Couldn't determine target architecture.");

    let target_dir = slang_dir
        .join("bin")
        .join(format!("{target_os}-{target_arch}"));
    if !target_dir.exists() {
        bail!(
            "Couldn't find slang libraries in directory: {}",
            target_dir.display()
        );
    }

    Ok(target_dir.join("release"))
}

#[cfg(feature = "from-source")]
fn build_from_source() -> PathBuf {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source_path = manifest_dir.join("slang");

    std::fs::canonicalize(cmake::build(source_path)).unwrap()
}

#[derive(Debug)]
struct ParseCallback {}

impl bindgen::callbacks::ParseCallbacks for ParseCallback {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        let enum_name = enum_name?;

        // Map enum names to the part of their variant names that needs to be trimmed.
        // When an enum name is not in this map the code below will try to trim the enum name itself.
        let mut map = std::collections::HashMap::new();
        map.insert("SlangMatrixLayoutMode", "SlangMatrixLayout");
        map.insert("SlangCompileTarget", "Slang");

        let trim = map.get(enum_name).unwrap_or(&enum_name);
        let new_variant_name = pascal_case_from_snake_case(original_variant_name);
        let new_variant_name = new_variant_name.trim_start_matches(trim);
        Some(new_variant_name.to_string())
    }
}

/// Converts `snake_case` or `SNAKE_CASE` to `PascalCase`.
fn pascal_case_from_snake_case(snake_case: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in snake_case.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c.to_ascii_lowercase());
        }
    }

    result
}
