//! Authored malformed native candidates are not live-service qualification.
use crate::{
    Distance,
    search_response::parse,
    writes::{hex, point_id},
};
use rom::{Key, json};
use rom_projection_core::{ProjectionProfile, VectorQuery};
use serde_json::Value;
fn profile() -> String {
    hex(
        &ProjectionProfile::new("fixture", "qdrant", "mapping", Some("model"))
            .unwrap()
            .fingerprint(),
    )
}
fn point(id: &str, revision: u64, score: f64) -> Value {
    json!({"id":point_id("documents",id),"version":1,"score":score,"payload":{"rom_kind":"documents","rom_id":id,"rom_profile":profile(),"rom_revision_hi":revision>>32,"rom_revision_lo":revision&0xffff_ffff,"rom_live":true}})
}
fn response(points: Vec<Value>) -> Value {
    json!({"status":"ok","result":{"points":points}})
}
#[test]
fn native_candidate_revisions_use_checked_halves_and_original_ids() {
    let query = VectorQuery::new(vec![1., 0., 0.], 2, 4).unwrap();
    let parsed = parse(
        "documents",
        &profile(),
        &query,
        Distance::Dot,
        &response(vec![
            point("Exact/ID", u64::MAX, 3.),
            point("second", 1 << 32, 2.),
        ]),
    )
    .unwrap();
    assert_eq!(parsed[0].key().id, "Exact/ID");
    assert_eq!(parsed[0].revision(), u64::MAX);
    assert_eq!(parsed[1].revision(), 1 << 32);
    assert!(
        parse(
            "documents",
            &profile(),
            &query,
            Distance::Dot,
            &response(vec![])
        )
        .unwrap()
        .is_empty()
    );
    for (field, value) in [
        ("rom_revision_hi", json!(4294967296_u64)),
        ("rom_revision_lo", json!(-1)),
        ("rom_revision_hi", json!(1.5)),
        ("rom_live", json!(false)),
        ("rom_profile", json!("changed")),
        ("rom_kind", json!("foreign")),
    ] {
        let mut p = point("a", 1, 3.);
        p["payload"][field] = value;
        assert!(
            parse(
                "documents",
                &profile(),
                &query,
                Distance::Dot,
                &response(vec![p])
            )
            .is_err()
        );
    }
    let mut p = point("a", 1, 3.);
    p["id"] = json!("wrong-native-id");
    assert!(
        parse(
            "documents",
            &profile(),
            &query,
            Distance::Dot,
            &response(vec![p])
        )
        .is_err()
    );
}
#[test]
fn native_distance_order_differs_from_similarity_and_must_be_finite() {
    let q = VectorQuery::new(vec![1., 0., 0.], 2, 4).unwrap();
    let ascending = response(vec![point("a", 1, 1.), point("b", 1, 2.)]);
    let descending = response(vec![point("a", 1, 2.), point("b", 1, 1.)]);
    assert!(parse("documents", &profile(), &q, Distance::Dot, &descending).is_ok());
    assert!(parse("documents", &profile(), &q, Distance::Dot, &ascending).is_err());
    for metric in [Distance::Euclid, Distance::Manhattan] {
        assert!(parse("documents", &profile(), &q, metric, &ascending).is_ok());
        assert!(parse("documents", &profile(), &q, metric, &descending).is_err());
        assert!(
            parse(
                "documents",
                &profile(),
                &q,
                metric,
                &response(vec![point("a", 1, -1.)])
            )
            .is_err()
        );
    }
    for score in [json!(null), json!("NaN"), json!(1e100)] {
        let mut p = point("a", 1, 3.);
        p["score"] = score;
        assert!(
            parse(
                "documents",
                &profile(),
                &q,
                Distance::Dot,
                &response(vec![p])
            )
            .is_err()
        );
    }
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![point("a", 1, 1.), point("b", 1, 1.)])
        )
        .is_ok()
    );
}
#[test]
fn native_candidates_reject_exclusions_duplicates_excess_and_extra_values() {
    let q = VectorQuery::new(vec![1., 0., 0.], 1, 1).unwrap();
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![point("a", 1, 1.), point("b", 1, 1.)])
        )
        .is_err()
    );
    let q = VectorQuery::new(vec![1., 0., 0.], 1, 4).unwrap();
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![point("a", 1, 1.), point("a", 1, 1.)])
        )
        .is_err()
    );
    let q = q
        .excluding(Key {
            kind: "documents".into(),
            id: "a".into(),
        })
        .unwrap();
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![point("a", 1, 1.)])
        )
        .is_err()
    );
    let mut p = point("b", 1, 1.);
    p["payload"]["rom_values"] = json!("private indexed value");
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![p])
        )
        .is_err()
    );
    let mut p = point("b", 1, 1.);
    p["vector"] = json!({"embedding":[1.,0.,0.]});
    assert!(
        parse(
            "documents",
            &profile(),
            &q,
            Distance::Dot,
            &response(vec![p])
        )
        .is_err()
    );
}

#[test]
fn cosine_candidates_accept_finite_descending_scores_and_empty_results() {
    let query = VectorQuery::new(vec![3., 4., 0.], 3, 4).unwrap();
    for points in [
        vec![],
        vec![
            point("same", 1, 1.),
            point("orthogonal", 1, 0.),
            point("opposite", 1, -1.),
        ],
    ] {
        assert!(
            parse(
                "documents",
                &profile(),
                &query,
                Distance::Cosine,
                &response(points)
            )
            .is_ok()
        );
    }
    assert!(
        parse(
            "documents",
            &profile(),
            &query,
            Distance::Cosine,
            &response(vec![point("low", 1, 0.), point("high", 1, 1.)])
        )
        .is_err()
    );
    let mut invalid = point("invalid", 1, 0.);
    invalid["score"] = json!(1e100);
    assert!(
        parse(
            "documents",
            &profile(),
            &query,
            Distance::Cosine,
            &response(vec![invalid])
        )
        .is_err()
    );
}
