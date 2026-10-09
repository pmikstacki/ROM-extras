//! Bound native metadata and metric order; never retain backend values or scores.
use crate::{Distance, writes::point_id};
use rom::Key;
use rom_projection_core::{SearchCandidate, TargetFailure, VectorQuery};
use serde_json::Value;
use std::collections::BTreeSet;
pub(crate) fn parse(
    kind: &str,
    profile: &str,
    query: &VectorQuery,
    metric: Distance,
    value: &Value,
) -> std::result::Result<Vec<SearchCandidate>, TargetFailure> {
    if value["status"].as_str() != Some("ok") {
        return Err(TargetFailure::Unknown);
    }
    let points = value["result"]["points"]
        .as_array()
        .ok_or(TargetFailure::Unknown)?;
    if points.len() > query.candidate_budget() || metric == Distance::Cosine {
        return Err(TargetFailure::Rejected);
    }
    let mut native_ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut previous = None;
    let mut candidates = Vec::new();
    for point in points {
        let payload = point["payload"]
            .as_object()
            .filter(|p| p.len() == 6)
            .ok_or(TargetFailure::Rejected)?;
        let key = Key {
            kind: payload
                .get("rom_kind")
                .and_then(Value::as_str)
                .ok_or(TargetFailure::Rejected)?
                .into(),
            id: payload
                .get("rom_id")
                .and_then(Value::as_str)
                .ok_or(TargetFailure::Rejected)?
                .into(),
        };
        if key.kind != kind
            || query.excluded() == Some(&key)
            || payload.get("rom_profile").and_then(Value::as_str) != Some(profile)
            || payload.get("rom_live").and_then(Value::as_bool) != Some(true)
        {
            return Err(TargetFailure::Rejected);
        }
        let hi = payload
            .get("rom_revision_hi")
            .and_then(Value::as_u64)
            .filter(|n| *n <= u32::MAX as u64)
            .ok_or(TargetFailure::Rejected)?;
        let lo = payload
            .get("rom_revision_lo")
            .and_then(Value::as_u64)
            .filter(|n| *n <= u32::MAX as u64)
            .ok_or(TargetFailure::Rejected)?;
        let candidate =
            SearchCandidate::new(key, (hi << 32) | lo).map_err(|_| TargetFailure::Rejected)?;
        let id = point["id"].as_str().ok_or(TargetFailure::Rejected)?;
        if id != point_id(&candidate.key().kind, &candidate.key().id)
            || !native_ids.insert(id)
            || !keys.insert((candidate.key().kind.clone(), candidate.key().id.clone()))
            || point.get("vector").is_some_and(|v| !v.is_null())
            || point.get("shard_key").is_some_and(|v| !v.is_null())
        {
            return Err(TargetFailure::Rejected);
        }
        let score = point["score"]
            .as_f64()
            .filter(|s| s.is_finite() && (*s as f32).is_finite())
            .ok_or(TargetFailure::Rejected)?;
        match metric {
            Distance::Dot => {
                if previous.is_some_and(|p| score > p) {
                    return Err(TargetFailure::Rejected);
                }
            }
            Distance::Euclid | Distance::Manhattan => {
                if score < 0. || previous.is_some_and(|p| score < p) {
                    return Err(TargetFailure::Rejected);
                }
            }
            Distance::Cosine => return Err(TargetFailure::Rejected),
        }
        previous = Some(score);
        candidates.push(candidate);
    }
    Ok(candidates)
}
