use std::process::Command;
use std::{env, fs};

use cached::proc_macro::cached;
use serde::{Deserialize, Serialize};

const CAIRO0_PIP_REQUIREMENTS_FILE: &str = "tests/requirements.txt";

/// Objects for simple deserialization of Cargo.toml to fetch the Cairo1 compiler version.
/// The compiler itself isn't actually a dependency, so we compile by using the version of the
/// cairo-lang-casm crate.
/// The choice of cairo-lang-casm is arbitrary, as all compiler crate dependencies should have the
/// same version.
/// Deserializes:
/// """
/// ...
/// [workspace.dependencies]
/// ...
/// cairo-lang-casm = VERSION
/// ...
/// """
/// where `VERSION` can be a simple "x.y.z" version string or an object with a "version" field.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum DependencyValue {
    // cairo-lang-casm = "x.y.z".
    String(String),
    // cairo-lang-casm = { version = "x.y.z", .. }.
    Object { version: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct CairoLangCasmDependency {
    #[serde(rename = "cairo-lang-casm")]
    cairo_lang_casm: DependencyValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceFields {
    dependencies: CairoLangCasmDependency,
}

#[derive(Debug, Serialize, Deserialize)]
struct CargoToml {
    workspace: WorkspaceFields,
}

#[cached]
/// Returns the version of the Cairo1 compiler defined in the root Cargo.toml (by checking the
/// package version of one of the crates from the compiler in the dependencies).
pub fn cairo1_compiler_version() -> String {
    let cargo_toml: CargoToml = toml::from_str(include_str!("../../../../Cargo.toml")).unwrap();
    match cargo_toml.workspace.dependencies.cairo_lang_casm {
        DependencyValue::String(version) | DependencyValue::Object { version } => version.clone(),
    }
}

/// Compiles a Cairo0 program using the deprecated compiler.
pub fn cairo0_compile(path: String, extra_arg: Option<String>, debug_info: bool) -> Vec<u8> {
    verify_cairo0_compiler_deps();
    let mut command = Command::new("starknet-compile-deprecated");
    command.arg(&path);
    if let Some(extra_arg) = extra_arg {
        command.arg(extra_arg);
    }
    if !debug_info {
        command.arg("--no_debug_info");
    }
    let compile_output = command.output().unwrap();
    let stderr_output = String::from_utf8(compile_output.stderr).unwrap();
    assert!(compile_output.status.success(), "{stderr_output}");
    compile_output.stdout
}

/// Compiles a Cairo1 program using the compiler version set in the Cargo.toml.
pub fn cairo1_compile(_path: String) -> Vec<u8> {
    todo!();
}

/// Verifies that the required dependencies are available before compiling; panics if unavailable.
fn verify_cairo0_compiler_deps() {
    // Python compiler. Verify correct version.
    let cairo_lang_version_output =
        Command::new("sh").arg("-c").arg("pip freeze | grep cairo-lang").output().unwrap().stdout;
    let cairo_lang_version_untrimmed = String::from_utf8(cairo_lang_version_output).unwrap();
    let cairo_lang_version = cairo_lang_version_untrimmed.trim();
    let requirements_contents = fs::read_to_string(CAIRO0_PIP_REQUIREMENTS_FILE).unwrap();
    let expected_cairo_lang_version = requirements_contents
        .lines()
        .nth(1) // Skip docstring.
        .expect(
            "Expecting requirements file to contain a docstring in the first line, and \
            then the required cairo-lang version in the second line."
        ).trim();

    assert_eq!(
        cairo_lang_version,
        expected_cairo_lang_version,
        "cairo-lang version {expected_cairo_lang_version} not found ({}). Please run:\npip3.9 \
         install -r {}/{}\nthen rerun the test.",
        if cairo_lang_version.is_empty() {
            String::from("no installed cairo-lang found")
        } else {
            format!("installed version: {cairo_lang_version}")
        },
        env::var("CARGO_MANIFEST_DIR").unwrap(),
        CAIRO0_PIP_REQUIREMENTS_FILE
    );
}
