# Palettes

Every `.toml` file in this folder is a palette Luna can use. They are **data, not
code**, so unlike tools they load at runtime: drop a file in, reopen the palette
picker, and it appears with a preview. No rebuild, no restart.

Files here come from two places: written by hand, or built with Luna's palette editor
tool, which saves into this folder. A palette made either way is the same thing.

Light and dark are **separate palettes**, not two variants of one. A palette declares
which it is through `appearance`, purely so the picker can group them sensibly.

## Format

```toml
luna_palette_version = 1        # format version, currently always 1

id          = "my_palette"      # unique, lowercase, matches the filename
name        = "My Palette"      # shown in the picker
description = "..."             # optional, shown under the name
appearance  = "dark"            # "dark" or "light", for grouping only
author      = "..."             # optional

[colors]
# every role below is required
```

## Roles

A palette assigns a colour to each **semantic role**. Tools ask for `warning`, never
for "orange", which is what lets one tool override a single role without knowing
anything about the palette in use.

| Group | Roles |
| --- | --- |
| Surfaces | `primary`, `secondary`, `tertiary`, `quaternary` |
| Text | `text`, `text_secondary`, `text_tertiary`, `text_quaternary` |
| Backgrounds | `background`, `background_secondary`, `background_tertiary`, `background_quaternary` |
| Borders | `border`, `border_secondary`, `border_tertiary`, `border_quaternary` |
| Meaning | `success`, `warning`, `error`, `info`, `danger` |
| States | `inactive`, `disabled` |
| Accent | `highlight` |

Colours are `#rrggbb` or `#rrggbbaa`.

Every role must be present. A palette missing one is reported in the picker and cannot
be applied, rather than silently falling back, because a missing role usually means a
typo in a role name.

## Overrides

The palette here is only the base layer. A tool can use a different palette entirely,
and can override individual roles on top of that:

```
tool colour override  ->  tool palette  ->  application palette
```

So the app can run Night Sky while the calendar runs Daylight with `warning` changed
to yellow, and nothing else is affected. Those overrides live in
`config/tools/<tool-id>.toml`, not here. Nothing but the palette editor ever writes to
this folder, so hand-written files are safe from being rewritten under you.

## Bundled palettes

| File | Appearance | Notes |
| --- | --- | --- |
| `night_sky.toml` | dark | The palette Luna was developed against. |
| `monochrome_gray.toml` | dark | Grayscale except the meaningful roles. |
| `pure_monochrome_gray.toml` | dark | Fully grayscale, roles differ by brightness. |
| `daylight.toml` | light | Light counterpart, so the picker ships with one of each. |
