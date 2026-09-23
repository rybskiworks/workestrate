# README artwork

The compact `workestrate-{light,dark}.svg` hero shows a fleet feeding individual
workload VMs. `operating-loop-{light,dark}.svg` illustrates the setup walkthrough.
The [installation hub](../install/README.md) links that workflow to the guide.

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
