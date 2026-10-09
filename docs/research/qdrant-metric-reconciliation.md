# Qdrant metric reconciliation requirements

Inspection date: 2026-10-09. This is an actual protocol experiment and a design requirement, not Rust adapter acceptance.

Qdrant [documents automatic Cosine normalization](https://qdrant.tech/documentation/manage-data/collections/).
The [pinned metric implementation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple.rs) performs different preprocessing for each metric.
The core's original vector contributes to deterministic document identity. Native retrieval need not return those original bytes.

The [execution record](../verification/qdrant-metric-observation-2026-10-09.json) contains sixteen actual point observations across four fresh persistent collections.
Verified HTTPS writes used the private writer key. Read-only retrieval established native values. The actual service reported version 1.19.2.
The script converted synthetic input to float32 before dispatch and required completed writes. It preserved all collections.

| Metric | `[3,4,0]` retrieval | Tested large finite vector | Tested zero and small vector |
| --- | --- | --- | --- |
| Cosine | `[0.6,0.8,0]` | Returned zeros | Preserved the tested float32 representations |
| Dot | Unchanged | Preserved float32 | Preserved float32 |
| Euclid | Unchanged | Preserved float32 | Preserved float32 |
| Manhattan | Unchanged | Preserved float32 | Preserved float32 |

The large input was the float32 representation of `[3e38,0,0]`. The small input represented `[1e-38,0,0]`.
JSON decimal spelling can differ while representing the same float32 value. Compare typed values, not decimal strings.
These observations do not establish all dimensions, CPU implementations, metrics, input ranges or query behavior.

Define metric-specific preparation and reconciliation before implementing the Rust target.
Preserve original approved content identity. Treat provider normalization as an explicit contract.
Do not silently clip values or infer the original sidecar from normalized native vectors.
Do not substitute a Dot-only final scope to avoid Cosine qualification.
Qualify zero, non-unit, extreme finite and dimension-boundary vectors through actual writes and queries.
Current protocol probes, checkpoint orchestration and public search tests remain separate evidence categories.
