# Unified Analyze JSON v4

Debtmap's CLI emits the v4 machine-readable contract with:

```bash
debtmap analyze . --format json --output report.json
```

The authoritative schema is
[`schemas/debtmap-output-v4.schema.json`](../schemas/debtmap-output-v4.schema.json), and the
minimal fixture is
[`tests/fixtures/output/unified-v4-minimal.json`](../tests/fixtures/output/unified-v4-minimal.json).

V4 keeps the v3 `metadata`, `summary`, and `items` shapes and adds a required `receipt`:

```json
{
  "format_version": "4.0",
  "metadata": { "project_root": "/workspace/project" },
  "receipt": {
    "analysis_target": "/workspace/project",
    "source_revision": { "commit": "0123456789abcdef0123456789abcdef01234567", "dirty": false },
    "reference_time": "2026-08-19T00:00:00Z",
    "policy": { "languages": ["rust", "python"] },
    "policy_fingerprint": "...",
    "evidence": {
      "coverage_requested": false,
      "coverage_loaded": false,
      "coverage_source_kind": null,
      "context_requested": false
    },
    "selection": { "top": 10, "tail": null },
    "execution": { "parallel": true, "jobs": 8, "multi_pass": true },
    "scope": {
      "discovered_files": 42,
      "analyzed_files": 42,
      "failed_files": 0,
      "omitted_by_limit": 0,
      "total_loc": 12000,
      "status": "complete"
    },
    "warnings": []
  },
  "summary": {},
  "items": []
}
```

The receipt distinguishes requested evidence from evidence actually loaded. Source revision is
present only when the target belongs to a readable Git worktree, and includes dirty-worktree state
so a commit hash is not presented as a complete content identity. Operational settings
such as Rayon job count are recorded separately from scoring policy. `policy_fingerprint` hashes
the normalized policy object, not raw arguments, environment variables, paths, or secrets.

Consumers must not interpret nullable receipt fields as zero. A `limited`, `partial`, or `unknown`
scope is not proof of complete project coverage; the warning list explains loaded-evidence and
scope limitations known at output time.

The compare command accepts both v3 and v4. It marks v3 comparability as `unknown`, and v4 reports
with different policy, evidence, selection, or target receipts as `incompatible`; incompatible
reports do not produce improvement claims.

Function items are findings. They do not contain generated refactoring recommendations in v4.
Optional nested evidence is omitted when it was not computed.

Consumers must reject unsupported `format_version` values. V3 remains frozen and readable for
compatibility; new CLI reports use v4.
