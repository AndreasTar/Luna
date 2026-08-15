# Tools

Every folder here is a tool. The build script discovers them, generates the wiring,
and Luna picks them up on the next build. Adding a tool means dropping a folder in:
no `Cargo.toml` edits, no registration list to update, no page chain to extend.

## What a tool folder looks like

```text
tools/<name>/
  manifest.toml    what the tool declares about itself
  mod.rs           defines `pub struct Tool`
  ui.slint         exports `component ToolPage`
```

All three are required, and the names are fixed. Declaring them in the manifest would
be three more fields that can disagree with reality.

The folder name becomes a Rust module name, so it must start with a letter or
underscore and contain only letters, digits and underscores.

## manifest.toml

```toml
id          = "luna.my_tool"     # stable, namespaced, never changes once shipped
name        = "My Tool"          # shown in the sidebar
version     = "0.1.0"
description = "One line, shown on the tool's page."
category    = "utilities"        # groups the sidebar

background  = false              # does it need to run with its page closed?

offers      = ["luna/text"]      # data it can hand to other tools
accepts     = ["luna/text"]      # data it will accept from them

state_schema = 1                 # bump to trigger a migration of its saved state
```

Only `id`, `name` and `version` are required. Unknown keys are rejected, so a typo
fails the build rather than being silently ignored.

**The id is load-bearing.** It keys the tool's config file, its data directory and its
scheduled jobs, so changing it after shipping orphans all three.

## mod.rs

```rust
use crate::tools::{ BoundTool, ToolView, ViewContext };
use crate::LunaAppUi;
use luna_core::ToolManifest;
use slint::Weak;

pub struct Tool {
    ui_handle: Weak<LunaAppUi>,
}

impl BoundTool for Tool {
    fn tool_id(&self) -> &'static str { return "luna.my_tool"; }
}

impl ToolView for Tool {
    fn manifest() -> ToolManifest {
        return ToolManifest::from_toml(include_str!("manifest.toml"))
            .expect("my_tool manifest.toml is malformed");
    }

    fn bind(ui_handle: Weak<LunaAppUi>, ctx: &ViewContext<'_>) -> Self {
        // ctx.paths is where the tool reads and writes. Open the shared database with
        // luna_core::Database::open(ctx.paths.database_file()), or take a folder of the
        // tool's own with ctx.paths.ensure_tool_data_dir. A path rather than an open
        // connection, because a rusqlite Connection is Send but not Sync.
        let _ = ctx;

        return Tool { ui_handle };
    }
}
```

The struct is always named `Tool`. It is reached as `my_tool::Tool`, so the folder
name already does the disambiguating.

Tools are compiled as modules of `luna_ui`, not as separate crates, so `crate::` here
refers to the app.

## ui.slint

```slint
import { PageComponent } from "../../luna_src/src/helpers/page_component.slint";

export component ToolPage inherits PageComponent {
    title: @tr("My Tool");
    description: @tr("One line about it.");
}
```

The component is always named `ToolPage`. The build script imports it under an alias
per tool, so the names never collide.

## Background services

A tool that needs to keep working with its page closed declares `background = true`
and implements `luna_core::ToolService`, which the registry starts when the tool is
enabled and drops when it is disabled. A tool that declares `background = true` but
registers no service factory fails at startup rather than quietly doing nothing.

The factory is returned from `ToolView::service`, which defaults to `None`:

```rust
fn service() -> Option<luna_core::ServiceFactory> {
    return Some(Box::new(|| Box::new(MyService::default())));
}
```

A factory rather than an instance, because disabling a tool drops its service and
re-enabling has to build a fresh one.

Do not declare it otherwise. An idle service is memory spent for nothing, and idle
footprint is the point of the background model.

**Registering scheduled work.** A service opens its own `luna_core::Scheduler` and
upserts jobs owned by its tool id. The running scheduler thread holds its jobs in
memory, so it will not see a write from another connection by itself; writing bumps a
revision in `_luna_meta` that the thread compares on each tick and reloads when it has
moved. That is what lets a job registered mid-session fire without a restart. Removing
a tool takes its jobs with it through `Scheduler::remove_tool`.

## Editor support

Tool sources live outside `luna_src/src`, so nothing would resolve them without help.
The build script writes `luna_src/src/tools/generated.rs`, a normal module file whose
`#[path]` declarations point back here. rust-analyzer reads it like any other source
file, so editing a tool gets completions, go-to-definition and inline errors.

That file is generated but **committed**, so a fresh clone works before the first
build. Do not edit it; the build script rewrites it whenever the contents of this
folder change. Adding a tool therefore shows up as a diff in it, which is intended.

**A brand new tool folder needs one build before the editor knows about it**, because
`generated.rs` does not mention it yet. A `cargo check` is enough and takes a few
seconds.

## What the build script checks

The build fails, naming the folder, if a tool has a malformed manifest, is missing one
of the three files, has a folder name that is not a usable module name, or declares an
id another tool already uses.
