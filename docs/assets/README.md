# README artwork

These are source SVGs, not screenshots. Both theme variants are committed so
GitHub can select them with `<picture>` while other renderers use the light image.
The root README links the hero to the rybskiworks organization.

| Pair | Purpose |
| :--- | :--- |
| `workestrate-{light,dark}.svg` | Project identity and a conceptual control-plane / individual-workload drawing. |
| `operating-loop-{light,dark}.svg` | Define policy, bound authority, delegate work, validate results. An operating philosophy, not a tested-capability badge. |

The visual language follows [Sketchbook](https://github.com/rybskiworks/sketchbook):
lowercase typography, a quiet dot field, thin technical linework, warm light and
deep dark surfaces, and a shared blue/red/blue accent. The drawings are specific
to Workestrate rather than copies of Sketchbook's branching exploration diagram.

Keep each pair's geometry and wording identical. Edit both palettes together.
Use explicit view boxes, readable system-font fallbacks, SVG titles/descriptions,
and meaningful README alt text. Keep essential information in Markdown as well.
The files need no scripts, animation, embedded fonts, external resources, or
`foreignObject`; the artwork is deliberately static.

After editing, parse each file as XML, render both themes at their intrinsic size
and at typical README widths, and check for clipped text or misaligned connectors.
Review the root README's `<picture>` paths and links. Exported raster previews
are review artifacts, not repository source files.
