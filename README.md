Readme is under development, check again later.

## What Luna is

A lightweight desktop helper that hosts many small tools — a number base converter, a
calendar, an image manipulator, with reminders, a PC health log, weather, file
converters and a password manager planned.

Luna runs in the background continuously, resumes without losing data or forgetting
scheduled work, and lets tools be enabled, disabled and connected to each other while
it is running. Tools are self-contained and compiled in: adding one means dropping a
folder into `tools/` and letting Luna rebuild itself.

Everything Luna writes stays in its own install folder, unless you explicitly point it
somewhere else.

The architecture is documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). It
describes the target design — most of it is not implemented yet.

## Repository layout

| Path | What it is |
| --- | --- |
| `luna_lib/` | Published crate `luna` — pure logic, no UI. Usable on its own. |
| `luna_src/` | The application: Slint UI shell and tool wiring. |
| `docs/` | Design documentation. |

## Building

Requires a Rust toolchain (edition 2021, rustc 1.88+).

```
cargo run -p luna_ui
```

## License

This project is licensed under the PolyForm Noncommercial License 1.0.0.  
You may use, modify, and share the software for noncommercial purposes only.
Full license document can be found in the [LICENSE](LICENSE) tab.

**Commercial use requires a separate license and explicit written permission from the author.**

To request a commercial license, contact Andreas Tarasidis via the contact info present on the [Github profile](https://github.com/AndreasTar).
