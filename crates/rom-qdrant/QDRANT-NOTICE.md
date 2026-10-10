# Qdrant arithmetic attribution

The safe references in `src/cosine.rs` derive the arithmetic sequence from Qdrant1.19.2.
Upstream authors: Qdrant contributors.
Upstream license: Apache License, Version2.0.
Source: https://github.com/qdrant/qdrant/tree/v1.19.2/lib/segment/src/spaces
License: https://github.com/qdrant/qdrant/blob/v1.19.2/LICENSE

ROM-extras replaces architecture intrinsics with safe scalar reference operations.
It adds bounded float64 input preparation and exact complete-candidate reconciliation.
These references do not establish native ARM or arbitrary compiler/floating-point-environment qualification.
