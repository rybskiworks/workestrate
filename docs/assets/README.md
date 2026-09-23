# README artwork

Edit these SVGs directly. GitHub selects the light or dark palette through
`<picture>`; other renderers use the light image. The root README links the hero
to the rybskiworks organization and the workflow to the
[setup guide](../getting-started.md).

| Pair | Purpose |
| :--- | :--- |
| `workestrate-{light,dark}.svg` | Project identity and a conceptual control-plane / individual-workload drawing. |
| `operating-loop-{light,dark}.svg` | Setup workflow: connect a fleet, inspect workloads, prepare credentials and host, then run. |

The visual language follows [Sketchbook](https://github.com/rybskiworks/sketchbook):
lowercase typography, a quiet dot field, thin technical linework, warm light and
deep dark surfaces, and a shared blue/red/blue accent. The drawings show
Workestrate's workload model and setup workflow.

Keep each pair's geometry and wording identical. Edit both palettes together.
Use explicit view boxes, readable system-font fallbacks, SVG titles/descriptions,
and meaningful README alt text. Keep essential information in Markdown as well.
The files need no scripts, animation, embedded fonts, external resources, or
`foreignObject`; the artwork is deliberately static.

After editing, parse each file as XML, render both themes at their intrinsic size
and at typical README widths, and check for clipped text or misaligned connectors.
Review the root README's `<picture>` paths and links. Exported raster previews
are review artifacts, not repository source files.
