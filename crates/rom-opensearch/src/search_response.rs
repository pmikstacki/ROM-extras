//! Strict complete native response decoding; values and scores never become candidates.
use crate::writes;
use rom::Key;
use rom_projection_core::{SearchCandidate, TargetFailure};
use serde_json::Value;
use std::collections::BTreeSet;
pub(crate) fn parse(
    response: &Value,
    physical: &str,
    profile: &str,
    kind: &str,
    budget: usize,
) -> Result<Vec<SearchCandidate>, TargetFailure> {
    let shards = &response["_shards"];
    if response["timed_out"].as_bool() != Some(false)
        || response.get("error").is_some()
        || response
            .get("terminated_early")
            .is_some_and(|v| v.as_bool() != Some(false))
        || shards["failed"].as_u64() != Some(0)
        || shards["total"]
            .as_u64()
            .is_none_or(|total| total == 0 || shards["successful"].as_u64() != Some(total))
    {
        return Err(TargetFailure::Unknown);
    }
    let hits = response["hits"]["hits"]
        .as_array()
        .filter(|hits| hits.len() <= budget)
        .ok_or(TargetFailure::Unknown)?;
    let mut candidates = Vec::new();
    let mut unique = BTreeSet::new();
    for hit in hits {
        let source = hit["_source"]
            .as_object()
            .filter(|s| s.len() == 5)
            .ok_or(TargetFailure::Unknown)?;
        let revision = source
            .get("rom_revision")
            .and_then(Value::as_u64)
            .filter(|r| *r > 0 && *r <= i64::MAX as u64)
            .ok_or(TargetFailure::Unknown)?;
        let identity = source
            .get("rom_id")
            .and_then(Value::as_str)
            .ok_or(TargetFailure::Unknown)?;
        if hit["_index"].as_str() != Some(physical)
            || hit["_version"].as_u64() != Some(revision)
            || source.get("rom_kind").and_then(Value::as_str) != Some(kind)
            || source.get("rom_profile").and_then(Value::as_str) != Some(profile)
            || source.get("rom_live").and_then(Value::as_bool) != Some(true)
            || hit["_id"].as_str() != Some(writes::id(kind, identity).as_str())
            || !unique.insert(identity)
        {
            return Err(TargetFailure::Unknown);
        }
        candidates.push(
            SearchCandidate::new(
                Key {
                    kind: kind.into(),
                    id: identity.into(),
                },
                revision,
            )
            .map_err(|_| TargetFailure::Unknown)?,
        );
    }
    Ok(candidates)
}
