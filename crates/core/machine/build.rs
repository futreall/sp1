fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }

    #[cfg(feature = "sys")]
    sys::build_ffi();
}

#[cfg(feature = "sys")]
mod sys {
    use std::{
        env, fs, os,
        path::{Path, PathBuf},
    };

    use pathdiff::diff_paths;

    /// The library name, used for the static library archive and the headers.
    /// Should be chosen as to not conflict with other library/header names.
    const LIB_NAME: &str = "sp1-core-machine-sys";

    /// The name of all include directories involved, used to find and output header files.
    const INCLUDE_DIRNAME: &str = "include";

    /// The name of the directory to recursively search for source files in.
    const SOURCE_DIRNAME: &str = "cpp";

    /// The warning placed in the cbindgen header.
    const AUTOGEN_WARNING: &str =
        "/* Automatically generated by `cbindgen`. Not intended for manual editing. */";

    pub fn build_ffi() {
        // The name of the header generated by `cbindgen`.
        let cbindgen_hpp = &format!("{LIB_NAME}-cbindgen.hpp");

        // The crate directory.
        let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

        // The output directory, where built artifacts should be placed.
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // The target directory that the cargo invocation is using.
        // Headers are symlinked into `target/include` purely for IDE purposes.
        let target_dir = {
            let mut dir = out_dir.clone();
            loop {
                if dir.ends_with("target") {
                    break Some(dir);
                }
                if !dir.pop() {
                    break None;
                }
            }
        };

        // The directory to read headers from.
        let source_include_dir = crate_dir.join(INCLUDE_DIRNAME);

        // The directory to place headers into.
        let target_include_dir = out_dir.join(INCLUDE_DIRNAME);

        // The directory to place symlinks to headers into. Has the fixed path "target/include".
        let target_include_dir_fixed = target_dir.map(|dir| dir.join(INCLUDE_DIRNAME));

        // The directory to read source files from.
        let source_dir = crate_dir.join(SOURCE_DIRNAME);

        let headers = glob::glob(source_include_dir.join("**/*.hpp").to_str().unwrap())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let compilation_units = glob::glob(source_dir.join("**/*.cpp").to_str().unwrap())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Tell Cargo that if the given file changes, to rerun this build script.
        println!("cargo::rerun-if-changed={INCLUDE_DIRNAME}");
        println!("cargo::rerun-if-changed={SOURCE_DIRNAME}");
        println!("cargo::rerun-if-changed=src");
        println!("cargo::rerun-if-changed=Cargo.toml");

        // Cargo build script metadata, used by dependents' build scripts.
        // The root directory containing the library archive.
        println!("cargo::metadata=root={}", out_dir.to_str().unwrap());

        // The include path defining the library's API.
        println!("cargo::metadata=include={}", target_include_dir.to_str().unwrap());

        // Generate a header containing bindings to the crate.
        match cbindgen::Builder::new()
            .with_pragma_once(true)
            .with_autogen_warning(AUTOGEN_WARNING)
            .with_no_includes()
            .with_sys_include("cstdint")
            .with_parse_deps(true)
            .with_parse_include(&[
                "sp1-stark",
                "sp1-primitives",
                "sp1-core-machine",
                "p3-baby-bear",
                "sp1-core-executor",
            ])
            .with_parse_extra_bindings(&["sp1-stark", "sp1-primitives", "p3-baby-bear"])
            .rename_item("BabyBear", "BabyBearP3")
            .include_item("MemoryRecord") // Just for convenience. Not exposed, so we need to manually do this.
            .include_item("SyscallCode") // Required for populating the CPU columns for ECALL.
            .include_item("SepticExtension")
            .include_item("SepticCurve")
            .include_item("MemoryLocalCols")
            .include_item("MEMORY_LOCAL_INITIAL_DIGEST_POS")
            .include_item("Ghost")
            .include_item("MemoryInitCols")
            .include_item("MemoryInitializeFinalizeEvent")
            .include_item("GlobalInteractionOperation")
            .include_item("GlobalInteractionEvent")
            .include_item("Poseidon2StateCols")
            .include_item("GlobalCols")
            .include_item("INTERACTION_KIND_GLOBAL")
            .with_namespace("sp1_core_machine_sys")
            .with_crate(crate_dir)
            .generate()
        {
            Ok(bindings) => {
                // Write the bindings to the target include directory.
                let header_path = target_include_dir.join(cbindgen_hpp);
                if bindings.write_to_file(&header_path) {
                    if let Some(ref target_include_dir_fixed) = target_include_dir_fixed {
                        // Symlink the header to the fixed include directory.
                        rel_symlink_file(header_path, target_include_dir_fixed.join(cbindgen_hpp));
                    }
                }
            }
            Err(cbindgen::Error::ParseSyntaxError { .. }) => {} // Ignore parse errors so rust-analyzer can run.
            Err(e) => panic!("{:?}", e),
        }

        // Copy the headers to the include directory and symlink them to the fixed include directory.
        for header in &headers {
            // Get the path of the header relative to the source include directory.
            let relpath = diff_paths(header, &source_include_dir).unwrap();

            // Let the destination path be the same place relative to the target include directory.
            let dst = target_include_dir.join(&relpath);

            // Create the parent directory if it does not exist.
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(header, &dst).unwrap();

            // We only need to symlink if we have a target.
            if let Some(ref target_include_dir_fixed) = target_include_dir_fixed {
                // Symlink the header to the fixed include directory.
                rel_symlink_file(dst, target_include_dir_fixed.join(relpath));
            }
        }

        // Use the `cc` crate to build the library and statically link it to the crate.
        let mut cc_builder = cc::Build::new();
        cc_builder.files(&compilation_units).include(target_include_dir);
        cc_builder.cpp(true).std("c++17");
        cc_builder.compile(LIB_NAME)
    }

    /// Place a relative symlink pointing to `original` at `link`.
    fn rel_symlink_file<P, Q>(original: P, link: Q)
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        #[cfg(unix)]
        use os::unix::fs::symlink;
        #[cfg(windows)]
        use os::windows::fs::symlink_file as symlink;

        let target_dir = link.as_ref().parent().unwrap();
        fs::create_dir_all(target_dir).unwrap();
        let _ = fs::remove_file(&link);
        let relpath = diff_paths(original, target_dir).unwrap();
        symlink(relpath, link).unwrap();
    }
}
