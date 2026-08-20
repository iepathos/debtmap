# Unified Analyze JSON v3 (Legacy)

This frozen contract remains readable for compatibility. New CLI reports use
[`JSON v4`](json-output-v4.md), which adds an analysis receipt.

```bash
debtmap analyze . --format json --output report.json
```

The checked JSON Schema is [`schemas/debtmap-output-v3.schema.json`](../schemas/debtmap-output-v3.schema.json),
and the minimal fixture is [`tests/fixtures/output/unified-v3-minimal.json`](../tests/fixtures/output/unified-v3-minimal.json).

## Envelope

```json
{
  "format_version": "3.0",
  "metadata": {
    "debtmap_version": "0.21.2",
    "generated_at": "2026-01-01T00:00:00Z",
    "project_root": null,
    "analysis_type": "unified"
  },
  "summary": {
    "total_items": 0,
    "total_debt_score": 0.0,
    "debt_density": 0.0,
    "total_loc": 0,
    "by_type": { "File": 0, "Function": 0 },
    "by_category": {},
    "score_distribution": { "critical": 0, "high": 0, "medium": 0, "low": 0 }
  },
  "items": []
}
```

Each item has a `type` discriminator of `File` or `Function`. Both variants contain `score`,
`category`, lowercase `priority`, nested `location`, `metrics`, and `impact`. Function items also
contain `debt_type`, `function_role`, and `dependencies`. Optional analysis fields are omitted when
the corresponding signal is unavailable.

Use `-vv` to include `scoring_details`; the default JSON omits those implementation details.
`--top` and `--tail` filter function location groups. Their summaries describe the emitted item
subset while retaining codebase-wide `total_loc` and cohesion.

Consumers must check `format_version` before parsing. Version 3.0 is frozen: every emitted shape
change requires a new format version and an immutable schema artifact. Incompatible changes bump
the major version.

V3 does not record effective policy, requested versus loaded evidence, selection settings, or
scope completeness. Do not infer that two v3 reports are directly comparable merely because both
parsed successfully.

The schema strictly freezes the envelope, discriminator, summary, locations, and metric fields.
Some deeply nested analysis payloads are intentionally opaque objects in 3.0; consumers that use
those payloads should deserialize them defensively. `pattern_details` is intentionally arbitrary
JSON supplied by language analyzers.

Reports contain codebase metadata, including paths, function names, call relationships, context
ranges, and git-derived aggregates, but not source bodies. Review or redact reports before sending
them outside your trust boundary.

## Common queries

```bash
jq '.summary.total_items' report.json
jq '.summary.total_debt_score' report.json
jq '.summary.debt_density' report.json
jq '.items[] | select(.priority == "high" or .priority == "critical")' report.json
jq '.items[] | select(.type == "Function" and .metrics.cyclomatic_complexity > 10)' report.json
jq -r '.items[].location.file' report.json | sort -u
```
