# tombi configuration for workestrate config repos.
# tombi 1.2.5+ — see https://tombi-toml.github.io/tombi/

toml-version = "v1.0.0"

[format.rules]
indent-width = 2
line-width = 100

[lint.rules]
dotted-keys-out-of-order = "warn"
tables-out-of-order = "warn"

[schema]
enabled = true
strict = true

[[schemas]]
path = "schemas/workestrate.schema.json"
include = ["workestrate.toml", "overrides.toml"]
