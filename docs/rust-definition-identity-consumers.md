# Definition identity in data-flow consumers

Rust data-flow records retain the source definition column in their internal
function identity. This keeps same-line implementations separate in purity,
mutation, I/O, and transformation data.

Display locations from public reports do not include a column. Data-flow getters
resolve those legacy queries through the existing call-graph index when a graph
is available. A unique definition remains accessible; tied same-line definitions
stay ambiguous. Queries carrying a column use exact record identities. Standalone
data-flow graphs without a call graph retain exact lookup behavior.

Internal serialized data-flow map keys use a `v2:` prefix followed by a JSON
function identity, or a JSON identity pair for transformations. These keys retain
columns, module paths, and qualified Rust names without separator collisions.
Deserialization also accepts the previous `file:name:line` keys and the previous
`left|right` transformation keys. Legacy keys have no source column. Invalid keys
produce a deserialization error instead of silently dropping records.

This changes internal data-flow serialization only. Public JSON v3 gains no
fields, and public display locations remain unchanged. This does not provide a
migration for older unversioned binary layouts. Source-line debt attribution
remains limited to the location information available in the report.

Regression coverage lives in `tests/data_flow_definition_identity.rs`: unique
and ambiguous legacy getter queries, separate same-line serialized identities,
transformation pairs, and legacy key decoding.

Data-flow setters canonicalize missing-column identities when the graph proves a
unique match, including both transformation endpoints. Exact-column hits remain
direct map lookups. On a miss, getters can recover legacy serialized facts only
when their missing-column identity uniquely denotes the requested definition.
Appending I/O migrates the matching legacy operations before adding the new one.
Ambiguous legacy facts remain inaccessible to either same-line definition.
`tests/data_flow_legacy_identity.rs` covers cached legacy extraction, legacy JSON,
mixed transformation identities, ambiguity, and preserving existing I/O on append.
