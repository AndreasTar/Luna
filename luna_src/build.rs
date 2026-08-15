//! Discovers the tools in `tools/` and generates the wiring for them.
//!
//! Adding a tool means dropping a folder into `tools/`. Nothing here or in
//! `src/tools/mod.rs` needs editing, and no `Cargo.toml` changes, because tools are
//! modules rather than crates.
//!
//! ## What a tool folder looks like
//!
//! ```text
//! tools/<name>/
//!   manifest.toml    what the tool declares about itself
//!   mod.rs           must define `pub struct Tool` implementing ToolView
//!   ui.slint         must export `component ToolPage`
//! ```
//!
//! The fixed names are deliberate. Declaring them in the manifest would be three more
//! fields that can disagree with reality, and there is no case for a tool wanting a
//! different layout.
//!
//! ## What gets generated
//!
//! * `src/tools/generated.rs` with a `#[path]` module per tool plus the registration
//!   and binding lists.
//! * `$OUT_DIR/tool_pages.slint` with a `ToolPages` component holding the page chain
//!   keyed by tool id, imported by `landing_page.slint` through an include path.
//!
//! The Rust half lands in the source tree rather than `OUT_DIR`, on purpose. Tool
//! sources are only reachable through those `#[path]` declarations, and
//! rust-analyzer does not reliably follow `include!(concat!(env!("OUT_DIR"), ...))`.
//! Generating a plain module file instead means editing a tool gets completions,
//! go-to-definition and inline errors like any other source file. It is written only
//! when its content actually changes, so builds do not churn the file's timestamp.
//!
//! A malformed or duplicated manifest fails the build here, with the offending file
//! named, rather than surfacing later as a confusing Rust or Slint error.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A tool found on disk.
struct DiscoveredTool {
    /// Folder name, which becomes the Rust module name.
    module: String,
    /// Tool id from the manifest, which keys the page chain and the registry.
    id: String,
    dir: PathBuf,
}

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is always set by cargo"));

    let tools_dir = manifest_dir
        .parent()
        .expect("luna_src always has a parent directory")
        .join("tools");

    // A new folder has to trigger a rebuild, otherwise dropping a tool in does nothing
    // until something else happens to change.
    println!("cargo:rerun-if-changed={}", slashed(&tools_dir));

    let tools = discover(&tools_dir);

    write_if_changed(
        &manifest_dir.join("src").join("tools").join("generated.rs"),
        &generate_rust(&tools),
    );
    write_if_changed(&out_dir.join("tool_pages.slint"), &generate_slint(&tools));

    let config =
        slint_build::CompilerConfiguration::new().with_include_paths(vec![out_dir.clone()]);

    slint_build::compile_with_config("src/ui/landing_page.slint", config)
        .expect("landing_page.slint failed to compile");
}

/// Finds every tool folder and checks it has the three files a tool needs.
fn discover(tools_dir: &Path) -> Vec<DiscoveredTool> {
    let entries = match std::fs::read_dir(tools_dir) {
        Ok(entries) => entries,
        // No tools directory at all is allowed: the app builds and shows its empty
        // state. Failing here would make a fresh checkout harder to get running.
        Err(_) => return Vec::new(),
    };

    let mut tools: Vec<DiscoveredTool> = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();

        if !dir.is_dir() {
            continue;
        }

        let module = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        let manifest_path = dir.join("manifest.toml");

        if !manifest_path.exists() {
            // A folder without a manifest is not a tool. Skipping rather than failing
            // keeps scratch directories from breaking the build.
            continue;
        }

        check_module_name(&module, &dir);
        require_file(&dir, "mod.rs", "must define `pub struct Tool`");
        require_file(&dir, "ui.slint", "must export `component ToolPage`");

        let id = read_manifest_id(&manifest_path);

        for existing in tools.iter() {
            if existing.id == id {
                fail(&format!(
                    "tools/{} and tools/{} both declare the id {:?}.\n\
                     Ids key config files, data directories and scheduled jobs, so they \
                     must be unique.",
                    existing.module, module, id
                ));
            }
        }

        for file in ["manifest.toml", "mod.rs", "ui.slint"] {
            println!("cargo:rerun-if-changed={}", slashed(&dir.join(file)));
        }

        tools.push(DiscoveredTool { module, id, dir });
    }

    // Sorted so the generated files are stable regardless of directory order, which
    // keeps rebuilds from churning for no reason.
    tools.sort_by(|a, b| a.module.cmp(&b.module));

    return tools;
}

/// Parses the manifest through the same type the app uses, so a manifest that builds
/// is a manifest that loads.
fn read_manifest_id(manifest_path: &Path) -> String {
    let text = match std::fs::read_to_string(manifest_path) {
        Ok(text) => text,
        Err(e) => fail(&format!("could not read {}: {e}", manifest_path.display())),
    };

    return match luna_core::ToolManifest::from_toml(&text) {
        Ok(manifest) => manifest.id,
        Err(e) => fail(&format!("{} is not a valid manifest: {e}", manifest_path.display())),
    };
}

fn check_module_name(module: &str, dir: &Path) {
    let valid_start = module
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');

    let valid_rest = module
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');

    if !valid_start || !valid_rest {
        fail(&format!(
            "{} is not a usable tool folder name.\n\
             The folder name becomes a Rust module, so it must start with a letter or \
             underscore and contain only letters, digits and underscores.",
            dir.display()
        ));
    }
}

fn require_file(dir: &Path, name: &str, why: &str) {
    if !dir.join(name).exists() {
        fail(&format!(
            "{} is missing {name}, which {why}.\n\
             Every tool folder needs manifest.toml, mod.rs and ui.slint.",
            dir.display()
        ));
    }
}

fn generate_rust(tools: &[DiscoveredTool]) -> String {
    let mut out = String::new();

    out.push_str(
        "// Generated by luna_src/build.rs from the contents of tools/. Do not edit.\n\
         //\n\
         // Committed rather than gitignored so a fresh clone has working editor support\n\
         // before the first build. Adding or removing a tool folder rewrites this file,\n\
         // which is why adding a tool shows up in a diff.\n\n",
    );

    for tool in tools {
        // Relative rather than absolute, so the file is the same on every machine and
        // can be committed. From src/tools/, three levels up is the repository root.
        let _ = writeln!(
            out,
            "#[path = \"../../../tools/{}/mod.rs\"]",
            tool.module
        );
        let _ = writeln!(out, "pub(crate) mod {};\n", tool.module);
    }

    out.push_str(
        "use super::{ BoundTool, ToolView, ViewContext };\n\n\
         /// Every tool as the registry takes it: the manifest, and the background half\n\
         /// for a tool that declares one.\n\
         pub(crate) fn registrations()\n\
         -> Vec<(luna_core::ToolManifest, Option<luna_core::ServiceFactory>)> {\n    return vec![\n",
    );
    for tool in tools {
        let _ = writeln!(
            out,
            "        ({}::Tool::manifest(), {}::Tool::service()),",
            tool.module, tool.module
        );
    }
    out.push_str("    ];\n}\n\n");

    out.push_str(
        "/// Binds every tool's callbacks and returns the bound views to be kept alive.\n\
         pub(crate) fn bind_all(\n    ui: &slint::Weak<crate::LunaAppUi>,\n\
         \x20   ctx: &ViewContext<'_>,\n) \
         -> Vec<Box<dyn BoundTool>> {\n    return vec![\n",
    );
    for tool in tools {
        let _ = writeln!(
            out,
            "        Box::new({}::Tool::bind(ui.clone(), ctx)),",
            tool.module
        );
    }
    out.push_str("    ];\n}\n");

    return out;
}

fn generate_slint(tools: &[DiscoveredTool]) -> String {
    let mut out = String::new();

    out.push_str("// Generated by luna_src/build.rs. Do not edit.\n\n");

    for tool in tools {
        let _ = writeln!(
            out,
            "import {{ ToolPage as ToolPage_{} }} from \"{}\";",
            tool.module,
            slashed(&tool.dir.join("ui.slint"))
        );
    }

    out.push_str(
        "\n// The active tool's page. Keyed by tool id rather than by index, because\n\
         // the sidebar contents change as tools are enabled and disabled.\n\
         export component ToolPages inherits Rectangle {\n    \
         in property <string> current-tool-id;\n\n    \
         VerticalLayout {\n",
    );

    for tool in tools {
        let _ = writeln!(
            out,
            "        if root.current-tool-id == \"{}\" : ToolPage_{} {{}}",
            tool.id, tool.module
        );
    }

    out.push_str("    }\n}\n");

    return out;
}

/// Writes only when the content differs.
///
/// Matters for the file in the source tree: rewriting it every build would change its
/// timestamp every build, which makes cargo and rust-analyzer redo work for nothing.
fn write_if_changed(path: &Path, contents: &str) {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return;
    }

    if let Err(e) = std::fs::write(path, contents) {
        fail(&format!("could not write {}: {e}", path.display()));
    }
}

/// Forward slashes, so generated Rust and Slint do not contain Windows path escapes.
fn slashed(path: &Path) -> String {
    return path.to_string_lossy().replace('\\', "/");
}

/// Stops the build with a message that names the problem and the file.
fn fail(message: &str) -> ! {
    panic!("\n\nLuna tool discovery failed.\n\n{message}\n\n");
}
