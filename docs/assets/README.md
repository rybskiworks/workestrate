# README artwork

The `workestrate-{light,dark}.svg` hero shows declarative configuration flowing
through Workestrate to individual agent and service microVMs.
`operating-loop-{light,dark}.svg` illustrates the setup walkthrough and links to
the guide from the [main README](../../README.md).

Edit the SVGs directly. GitHub selects the palette through `<picture>`; other
renderers use the light image. Keep both variants' geometry and wording in sync.

The artwork uses the existing blue/red/blue accent, quiet surfaces, and thin
linework. The README's centered `<samp>` text follows the restrained typography of
the [rybskiworks profile](https://github.com/rybskiworks).

Use explicit view boxes, readable system fonts, SVG titles/descriptions, and
meaningful alt text. Keep essential information in Markdown too. Assets are
static, with no scripts, embedded fonts, external resources, or `foreignObject`.

After editing, parse each SVG as XML and render both themes at intrinsic size and
typical README widths. Inspect text fit and connectors, then check the embedding
paths and links. Keep raster previews outside the repository.
