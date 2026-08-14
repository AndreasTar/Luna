# Luna - Architecture

Status: **design agreed, not yet implemented.** Last updated 2026-08-14.

This document describes the target architecture for Luna. The current codebase does
not implement most of it yet; see [Implementation roadmap](#13-implementation-roadmap)
for the order of work and [Current state](#14-current-state-vs-target) for the gap.

---

## 1. What Luna is

A lightweight desktop helper that hosts many small tools. It runs in the background
continuously, survives restarts without losing data, and lets tools be enabled,
disabled and connected to each other at runtime.

Current tools: number base converter, calendar (UI only), image manipulator (not
compiled). Planned: reminders, PC health log, weather, file converters, password
manager, image to ascii, vehicle maintenance log.

### Design goals

1. **Tools behave like plugins.** Self-contained, independently enable/disable-able
   at runtime, added by dropping a folder into the project.
2. **Always available.** Runs in the background; closing is deliberate, not incidental.
3. **Never loses data.** State survives crashes, kills and restarts. Timers and
   scheduled work resume correctly.
4. **Tools connect arbitrarily.** A tool can hand data to any other tool that accepts
   it, without either knowing the other exists at compile time.
5. **Portable.** Everything lives in the install folder. Nothing is written elsewhere
   unless the user explicitly says so.
6. **Low memory.** Idle footprint matters, because the app is always running.

### Non-goals

These were considered and explicitly rejected. Recorded so they don't get relitigated.

| Rejected | Why |
| --- | --- |
| Runtime-loaded plugins (dynamic libs / WASM) | Tools are compiled in. Removes ABI versioning, sandboxing and a whole class of failure. |
| Downloading tools from the internet | Nothing executes that the user didn't compile themselves. Collapses the threat model. |
| `%APPDATA%` / system config dirs | Portable by requirement. See section 7. |
| SpacetimeDB | It is a server for real-time multiplayer state sync. Wrong shape, and its footprint fights goal 6. Using `rusqlite` instead. |
| Capability sandboxing / permission prompts | Unnecessary once all code is first-party and compiled. Manifests remain, but describe *intent*, not a security boundary. |

---

## 2. Process model

Two processes, because Windows cannot overwrite a running executable and the
add-a-tool flow must be able to replace the binary.

```
luna_launcher.exe      supervisor, the thing the user starts
        |
        +-- spawns --> luna_app.exe      the real application
        |
        +-- on rebuild request:
              wait for exit -> cargo build -> swap binary -> relaunch
              on build failure -> keep previous binary -> relaunch -> report error
```

`luna_launcher` is deliberately tiny and changes rarely: spawn, watch, rebuild, swap,
relaunch. It holds no application logic, so a broken tool can never make it
unstartable.

**Binary swap safety.** `luna_app.exe` is kept alongside `luna_app.prev.exe`. A build
failure or a failed launch of the new binary rolls back to `.prev`. Adding a tool that
does not compile must never leave the user without a working app.

**Exit codes** are the app to launcher channel:

| Code | Meaning |
| --- | --- |
| `0` | Normal exit. Launcher exits too. |
| `10` | Restart requested. Relaunch same binary. |
| `11` | Rebuild requested. Compile, swap, relaunch. |

---

## 3. Adding a tool

1. User drops a tool folder into `tools/`.
2. App notices on next startup or window focus (a poll, not a filesystem watcher,
   because there is no urgency and a watcher costs memory).
3. App prompts: *"New tool `foo` found. Luna needs to rebuild and restart to use it."*
4. On confirm: app persists all state, exits with code `11`.
5. Launcher runs `cargo build --release`, swaps binaries, relaunches.
6. App restores state, restores the previously active page, reports the new tool.

**Rebuild cost is real.** A measured incremental build of `luna_ui` on this machine
took about 69 seconds. Tools are therefore separate crates, so adding one recompiles
that crate plus the shell rather than the entire dependency graph.

**`Cargo.lock` must be committed.** The rebuild happens on the user's machine at an
arbitrary later date; without a lockfile, dependency resolution can drift and a
rebuild that worked yesterday can fail today.

### Machines without a Rust toolchain

The rebuild flow requires `cargo` and a linker. This is fine for the author's own
machine and unacceptable as a hard requirement for anyone else. Resolution:

**Adding tools is a developer-mode feature, gated on toolchain presence.** At startup
the app probes for a working `cargo`. If absent, the tool-management UI shows the
installed tools and their enable/disable switches as normal, but the "add tool" path
is disabled with an explanation. Everything else works identically.

For those machines, tools arrive the ordinary way: **a new released build of Luna with
the tools already compiled in.** This is just normal software distribution, and it
means the architecture is identical in both cases. Only the *availability* of the
rebuild path differs.

Options deliberately not taken:

- *Bundling a toolchain*, roughly 0.7 to 1.5 GB with a linker. Fatal to goal 6.
- *Prompting to install rustup*, viable as an on-ramp, about 1 GB and a network
  dependency. Can be offered as a link; not automated.
- *A build server producing custom binaries*, which works but needs infrastructure
  that a single-developer project should not take on.

---

## 4. Crate layout

```
luna_lib/          published crate `luna`, pure logic, no UI, no IO
luna_core/         host services: registry, storage, scheduler, ports
tools/<name>/      one crate per tool: manifest + Rust + .slint
luna_src/          Slint shell + codegen build.rs  (binary `luna_app`)
luna_launcher/     supervisor                      (binary `luna_launcher`)
```

**`luna_lib` stays publishable**, which means it stays pure: no Slint, no filesystem,
no host types. It holds `number_converter`, `color_format_converter`,
`img_manipulator`, plus two new modules that are genuinely reusable and make it a
coherent crate rather than a grab-bag:

- `rules`: recurrence expansion, guard AST, urgency evaluation (section 8). Pure
  functions over a rule and an event history; no clock, no storage.
- `palette`: semantic colour roles, override resolution, contrast validation
  (section 9).

The *scheduler runtime* (threads, wakeups, persistence) lives in `luna_core`. Only the
pure evaluation lives in `luna_lib`.

---

## 5. The tool model

### 5.1 Two halves

The single biggest change from the current code. A tool splits in two:

**`ToolService`** is created at startup if the tool is enabled. It owns the tool's
state, handles scheduled work and inbound messages, never touches Slint, and runs off
the UI thread. It is **optional**: a base converter needs no background half and should
not get one.

**`ToolView`** is created lazily when the user opens the page, and destroyed on
navigate away. It talks to its service over a channel. Only the active view is
allocated.

This is what makes a reminder fire while the image editor is on screen, and what keeps
idle memory proportional to *background* tools rather than *installed* tools.

Both are boxed trait objects held by the registry, so tools live in a homogeneous
collection. (The pre-Slint `Vec<Box<dyn ToolPage>>` was closer to this than the
current `WidgetTrait`, which returns `Self` and cannot be collected.)

### 5.2 Manifest

Every tool ships a manifest, read by `build.rs` at compile time and by the registry at
runtime. It is a description of intent, not a security boundary.

```toml
id      = "luna.img_manipulator"     # stable, namespaced, never changes
name    = "Image Manipulator"
version = "0.3.0"
icon    = "icons/img.svg"
category = "media"

background = false      # does it need a ToolService?

offers  = ["luna/image"]                      # section 10
accepts = ["luna/image", "luna/file-path"]

state_schema = 1        # bumping this triggers a migration hook
```

### 5.3 Lifecycle

```
compile time   build.rs discovers tools/*, generates registry + slint imports
startup        registry constructed; enabled tools get a ToolService
user opens     ToolView constructed, UI state restored if within TTL (7.3)
user leaves    UI state snapshotted, ToolView dropped
disable        ToolService dropped; state flushed; tool stays compiled in
shutdown       shutdown hook (section 11) -> flush -> exit
```

Note that **enabled is not the same as compiled in**. Everything compiled is present;
enabling is a runtime toggle that controls whether a service runs and whether the tool
appears in the sidebar and in port lookups.

---

## 6. Build-time codegen

`luna_src/build.rs` scans `tools/`, parses each manifest, and generates:

1. **Rust**: the module tree, the registration list, and a `TOOLS` table the registry
   consumes at startup.
2. **Slint**: an aggregate file importing each tool's UI component and mapping tool
   id to component, replacing the hand-written `if current-item == N : XUI {}` chain
   in `landing_page.slint`.
3. **Sidebar model**: driven by the registry at runtime, not the literal array
   currently in `landing_page.slint:51`.

`slint_build` accepts include paths, so the generated `.slint` can live in `OUT_DIR`
and still resolve imports into tool directories. Emit
`cargo:rerun-if-changed=tools/` so a new folder triggers regeneration.

**Slint globals do not scale here.** Every global must be re-exported from the root
`.slint` file to be visible to Rust, which is exactly why the calendar's
`Global_Calendar_Callback` is currently unreachable from Rust while
`Global_NumberConversion_Callback` works. Per-tool globals would require the
generated file to re-export N globals and would keep reintroducing this bug. Tools
instead expose callbacks and properties **on their own component instance**, which the
generated glue wires up. One shared global remains, for theme (section 9).

---

## 7. Storage

### 7.1 Location

Everything lives beside the executable. Nothing is written to `%APPDATA%`, the
registry, or any system location.

```
<install>/
  luna_launcher.exe
  luna_app.exe
  luna_app.prev.exe
  config/
    app.toml                 global settings, palette selection, data_root override
    tools/<tool-id>.toml     per-tool settings
  data/
    luna.db                  SQLite
    tools/<tool-id>/         per-tool blobs (images, exports)
  logs/
  palettes/                  palette files, read at runtime
  tools/<name>/              tool sources (developer mode)
```

**`config/` is the anchor and is never redirected**, because it holds the file that
would record the redirection. Only the data root moves.

**Resolution order for the data root:**

1. `data_root` in `config/app.toml`, if the user set one.
2. Otherwise `<install>/data`.

There is no silent third fallback. A configured data root that turns out to be
unusable falls back to the default *and reports it*, rather than writing the user's
data somewhere they did not ask for.

If the install directory itself is not writable, which happens under `Program Files`
without elevation, Luna **cannot start portably and says so.** Prompting for an
alternative location is not possible here: there would be nowhere to record the
answer that does not violate goal 5. The user moves the install or elevates.

Worth warning about at startup: an install folder inside OneDrive or Dropbox can
corrupt a live SQLite file through sync.

### 7.2 Database

`rusqlite` with the `bundled` feature: one file, no external process, about 1 MB, WAL
mode for crash safety. It holds structured data such as calendar events, reminder
rules, the event log, health samples and password entries.

Config stays as TOML files: small, human-editable, diffable. Large binary artefacts
stay as files under `data/tools/<id>/`, not blobs in the DB.

**Atomic writes for every file write**: temp file, fsync, rename. An app designed
never to close will be killed eventually; a half-written config must not be possible.

**Autosave policy**: debounced about 500 ms after a change, plus on navigate-away, on
window hide, and periodically. Save-on-exit alone is worthless here.

### 7.3 UI state vs. user data

Kept in **separate stores**, because they have different durability requirements.

User data is precious. UI state, meaning scroll position, unsaved draft text, selected
layer and expanded panels, is disposable: if it fails to load or its schema drifted, it
is dropped silently. Corrupt UI state must never be able to damage user data.

Per-tool settings control it:

```toml
remember_ui_state = true
ui_state_ttl      = "7d"     # "session" | duration | "forever" | "never"
```

**Image editor specifically**: persist the *source image path and the filter stack
only*, never the rendered result. On return, re-decode and re-apply. Decoded images
are dropped entirely when the view is destroyed. This is the single largest memory
lever in the app.

---

## 8. Scheduling

One host scheduler, used by **every** tool, not a feature of a reminders tool. A PC
health log sampling hourly and a backup running at 02:00 register the same way a
reminder does.

### 8.1 Two kinds of scheduled item

**Instants** fire at a moment in time.

**Windows** have a state that evolves continuously and escalates in urgency.

### 8.2 Instants: generator plus guard

Every instant-based rule decomposes into a **temporal generator** plus an optional
**guard**.

The generator produces candidate instants from the clock alone. This is RFC 5545
`RRULE` territory, so use the `rrule` crate rather than inventing syntax. It covers
"second Sunday of the month", intervals, counts and until-dates, and it gives `.ics`
import/export nearly free for the calendar later.

The guard is a boolean predicate over **event history**, evaluated only when a
candidate arrives. If false, the candidate is skipped silently.

> *"If reminder A fired yesterday, remind me today at 12am"*
> equals generator `daily at 00:00` plus guard `fired(A, within: yesterday)`

This keeps the scheduler dumb, since it only ever handles candidate instants, while
allowing arbitrary conditions. Guard vocabulary is a small **typed, serialisable AST**,
not a scripting language:

```
fired(rule, window) | not_fired(rule, window)
acknowledged(rule, window) | snoozed(rule, window)
count(rule, window) >= n
completed(task, window)
and / or / not
```

Guards read only *past* events at discrete instants, so mutual dependencies cannot
infinite-loop. Detect and warn about them anyway, and cap how far back a guard window
may look so evaluation stays cheap.

### 8.3 Windows: soft deadlines that escalate

For work that should happen *sometime within a period*, becoming more urgent as the
period elapses. Vehicle maintenance is the motivating case: *"check your oil every
3 months."*

A window task has an **anchor**, a **shape**, and a derived **urgency**.

```
anchor = FixedSchedule(rrule)             # every 1st of the month
       | RollingFromCompletion(duration)  # 3 months after it was last done

shape  = { soft_start, target, hard_end }  # offsets from the anchor
```

The `RollingFromCompletion` anchor is what makes maintenance work correctly: the next
window starts from when you *actually did it*, not from a fixed calendar.

Urgency is a pure function of where `now` sits in the window:

| State | Range | Presentation |
| --- | --- | --- |
| `Dormant` | before `soft_start` | hidden |
| `Upcoming` | `soft_start` to `target` | listed quietly in the Upcoming panel |
| `Due` | at `target` | normal notification |
| `Overdue` | `target` to `hard_end` | persistent indicator, intensifying |
| `Critical` | past `hard_end` | prominent, repeating |

It also yields a normalised `0.0..1.0` intensity, which the UI interpolates across
palette roles (`info` to `warning` to `error`) so escalation is visible at a glance and
still respects per-tool theming (section 9).

A window does not "fire" once, it has state. Implementation unifies the two kinds:
each window registers threshold instants at `soft_start`, `target` and `hard_end`, and
exposes a queryable state that any tool can read. The UI recomputes urgency on a slow
tick (a minute is ample) and on view open.

**Completion** is logged as an event, which both feeds guards (8.2) and rolls the
next window for `RollingFromCompletion` anchors.

### 8.4 External progress sources (extension point)

The oil-change rule is really *"every 3 months **or** 5000 km, whichever comes first."*
Luna cannot know about kilometres, but it can ask.

A window task may declare an optional `progress_source`: a named metric plus a
threshold. The host asks the owning tool what the current progress toward this task is,
as a `0.0..1.0` value, and escalates on whichever of time or progress is further along.
If the providing tool is missing or disabled, the condition is skipped and time alone
applies, which is the same graceful degradation as ports (section 10).

This is declared now and implemented when a tool needs it. It costs nothing to leave
the hook in the model and makes the vehicle-maintenance tool possible later without a
redesign.

### 8.5 Correctness requirements

- **Store absolute deadlines, never "in N seconds."** Compute overdue work on resume.
- **Store recurrence rules, not the next instant.** A stored timestamp breaks across
  DST. Rules are held in local time with a timezone id and materialised to UTC.
- **Materialise only the next occurrence** per rule; re-materialise after each fire.
  Never expand an infinite series.
- **Per-job catch-up policy**: `fire_late`, `collapse_to_one`, or `skip`. Without it,
  opening Luna after a week produces fourteen stacked daily reminders.
- **Snooze and acknowledge are first-class**, because guards reference them.
- Handle system sleep and wall-clock changes; monotonic clocks behave differently
  across suspend on Windows.

---

## 9. Theming

### 9.1 Palettes are droppable files

A palette is a TOML file in `<install>/palettes`, not compiled-in data. Because a
palette is **data rather than code**, it does not need the rebuild flow that tools
need (section 3): drop the file in, reopen the picker, and it is there with a preview.

This is the one place where "drop a file in and it just works" applies literally, and
it is worth being explicit about why. Tools contribute behaviour and UI, which on this
architecture means Rust and Slint, which means compilation. Palettes contribute only
values, which can be parsed at runtime with no such cost.

The picker lists every palette it finds, renders a swatch preview from the file, and
applies it on click.

The folder is read by the picker and **written by the palette editor tool**, which
saves what the user builds straight into it, so a palette created in Luna is
indistinguishable from one dropped in by hand. Those writes go through the atomic path
like everything else. Per-tool overrides are still recorded in `config/`, never by
editing a palette file, so the editor is the only thing that ever writes here.

A palette missing a role is listed but not applicable, and says which role is missing.
Silently defaulting would turn a typo in a role name into a colour that is subtly wrong
somewhere far from the mistake.

### 9.2 Light and dark are separate palettes

Not two variants of one. A palette declares `appearance = "light" | "dark"` purely so
the picker can group them; there is no companion mode toggle, and `AppConfig` carries
no `dark_mode` flag. A user who wants light picks a light palette.

This keeps the model flat: one active palette id, one set of roles, no variant axis
multiplying every override.

### 9.3 Roles and resolution

Roles come from the existing `LunaPallete`, which already had the right abstraction:
primary, text, background and border families, plus `success` `warning` `error` `info`
`danger`, `inactive` `disabled` `highlight`. Its four palettes were salvaged into
`palettes/` when the iced code was removed.

Three levels of override, resolved per `(tool, role)`:

```
effective(tool, role) =
      tool_color_override(tool, role)   // warning -> yellow, in tool A only
   ?? tool_palette(tool)[role]          // tool A uses palette Y
   ?? app_palette[role]                 // app uses palette X
```

Resolution happens in Rust and produces a **flat resolved palette**, pushed into a
single Slint global `Theme` when the active tool changes. Tool components just read
`Theme.warning` and never know an override exists. One active palette suffices because
one tool page is visible at a time; this would become a per-component property only if
two tools were ever shown side by side.

Persisted form stays small: an app palette id in `config/app.toml`, and per tool an
optional palette id plus a sparse `role -> colour` map in `config/tools/<id>.toml`.

**Contrast validation** lives in `luna_lib`: when a user overrides a role, check it
against the resolved background and warn if it falls below a readable ratio. Pure
function, pairs naturally with `color_format_converter`, and it is the kind of thing
that justifies publishing the crate.

---

## 10. Inter-tool connections (ports)

Tools declare typed `offers` and `accepts` in their manifest (5.2). Types come from a
small shared vocabulary: `luna/image`, `luna/text`, `luna/color`, `luna/file-path`,
`luna/number`.

**The sender never names the receiver.** The image editor asks the host which tools
accept `luna/image`; the host filters to installed-and-enabled tools and returns a
list; the editor renders a "Send to" submenu from it. If the ascii converter is not
installed or is disabled, it simply is not in the list. Nothing breaks, and there is no
compile-time coupling between the two tools.

Large payloads are not pushed through the bus. They go into a content-addressed blob
store and travel as `Arc`-backed handles, so sending a 50 MP image between tools costs
a refcount.

---

## 11. Shutdown, restart, and what stops when closed

Closing is deliberate. The tray keeps Luna resident; the window close button hides it.
A real **Quit** and a **Restart** are explicit actions.

**The quit prompt aggregates scheduled work across every tool**, not just reminders,
because the scheduler is a host service (section 8):

```
Luna is closing. While closed:
  - 3 reminders will not fire      next: today 12:00
  - PC Health Log will not sample  next: in 10 minutes
  - Backup                         next: today 02:00
Oil change is Overdue (11 days).
  [ Quit ]  [ Minimise to tray ]  [ Cancel ]
```

This is the one piece of information that actually changes the user's decision, so it
is worth building properly rather than showing a generic confirmation.

**Tools may object to shutdown.** A shutdown hook returns
`Allow | NeedsConfirmation(message) | RequestDelay(reason)`, which is enough for "file
conversion in progress, about 30s remaining" to hold the door briefly, and for an
unsaved editor to say so.

**Restart** reuses the launcher path minus the compile step (exit code `10`).

---

## 12. Threading

The Slint event loop owns the UI thread. Host services (registry, scheduler, ports,
storage) run off it and communicate over channels, with results marshalled back via
`slint::invoke_from_event_loop`.

This decouples "the app is running" from "a window exists", which is what allows the
window to be **destroyed rather than hidden** when minimised to tray. Services keep
ticking with no UI allocated, which is the main idle-memory lever.

All long operations (image filters, file conversion, DB queries) run off the UI
thread. The existing `main.rs:19` TODO already notes this.

Async runtime: start with plain threads and channels. Adopt `tokio` only when the
networked features such as weather actually land, rather than paying for it now.

---

## 13. Implementation roadmap

Each step is independently shippable, and each exercises the interfaces the later
steps depend on.

1. **Paths + storage + atomic saves.** *Done, `luna_core` v0.1.0.* Portable layout,
   data-root resolution, atomic writes, config load/save with quarantine-on-corrupt,
   SQLite bootstrap with a migration runner.
2. **Registry + manifest + service/view split + codegen `build.rs`.** Built-ins move
   behind the tool interface; sidebar driven by the registry. *Runtime enable/disable
   works.* Also removes the `Global_Calendar_Callback` bug by construction.
3. **`luna_launcher` + rebuild-on-detect + resume.** The add-a-tool model becomes real.
4. **Scheduler + rule engine + event log.** Instants, guards, windows, escalation,
   catch-up. Reminders survive restarts.
5. **Palette loading, picker and resolution + `Theme` global.** Low risk, immediately
   visible; can slot in anywhere after step 2.
6. **Ports.** Cheap once the registry exists.
7. **UI state TTL, memory tuning, image editor re-apply-on-return.**

Steps 1 and 2 are what make step 3 safe. Building the launcher first would mean having
nothing meaningful to resume.

---

## 14. Current state vs. target

What exists today, for orientation.

**Working:** the workspace split (`luna_lib`, `luna_core` and `luna_src`), storage and
config through `luna_core`, the base converter end-to-end, `number_converter` and
`color_format_converter` as solid documented library modules, and the calendar's UI
layout. The stale iced and egui files have been removed, with the `LunaPallete` work
salvaged into `palettes/`.

**Known gaps and defects**, all superseded or fixed by the work above:

- `Global_Calendar_Callback` is not re-exported from `landing_page.slint`, so every
  calendar click handler is unreachable from Rust. Fixed by section 6.
- `calendar_ui.slint:4` imports the same global three times.
- `luna_src/src/helpers/positioner.rs` is the last egui-era file. It compiles and is
  declared in `helpers/mod.rs`, but nothing uses it and the only function that consumed
  its types is commented out. Keep with an explanatory comment, or delete.
- `color_format_converter::convert_vec_color_model` derives channel count from
  `format as usize % 2`, which is only correct for the first 12 RGB permutations and
  wrong for `Gray`, `GrayA`, and the CMYKA/HSLA variants. `channel_count()` already
  exists and is correct.
- `channel_order`: `HLS` maps to `[H, L, A]` and `HLSA` to `[H, L, A, A]`; both should
  carry `S`.
- `from_rgb_to_hsl` does `u8` arithmetic that underflows and panics in debug builds.
  `from_cmyk_to_rgb_integer` can underflow when `c` and `k` are both high.
- The module doctest calls `from_cmyk_to_rgb_checked`, which does not exist. It only
  passes CI because `color_format_converter` is not a default feature.
- `convert_from_decimal` still carries the `tries: u8 = 64` debug counter, and `u32`
  caps input around 4.29e9.
- Calendar month grid does not offset the first day by its weekday, and
  `get_monthly_day_count` returns 28/29 for any unlisted month, including 0 and 13.
