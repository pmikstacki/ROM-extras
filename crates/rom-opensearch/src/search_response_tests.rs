//! Complete-response protocol negatives independent of a native server.
use crate::{search_response::parse, writes};
use serde_json::{Value, json};
fn response() -> Value {
    json!({"timed_out":false,"_shards":{"total":1,"successful":1,"skipped":0,"failed":0},
        "hits":{"hits":[{"_index":"generation","_id":writes::id("docs","a/雪"),"_version":1,
            "_source":{"rom_kind":"docs","rom_id":"a/雪","rom_revision":1,"rom_profile":"profile","rom_live":true}}]}})
}
fn rejected(value: &Value, budget: usize) {
    assert!(parse(value, "generation", "profile", "docs", budget).is_err());
}
#[test]
fn complete_response_keeps_original_key_and_revision() {
    let result = parse(&response(), "generation", "profile", "docs", 1).unwrap();
    assert_eq!(result[0].key().id, "a/雪");
    assert_eq!(result[0].revision(), 1);
}
#[test]
fn timeout_partial_and_early_termination_are_not_empty_success() {
    for (pointer, value) in [
        ("/timed_out", json!(true)),
        ("/timed_out", json!(null)),
        ("/_shards/failed", json!(1)),
        ("/_shards/successful", json!(0)),
        ("/_shards/total", json!(0)),
    ] {
        let mut changed = response();
        *changed.pointer_mut(pointer).unwrap() = value;
        rejected(&changed, 1);
    }
    let mut changed = response();
    changed["terminated_early"] = json!(true);
    rejected(&changed, 1);
}
#[test]
fn foreign_identity_profile_and_hidden_source_fail_closed() {
    for (pointer, value) in [
        ("/hits/hits/0/_index", json!("alias")),
        ("/hits/hits/0/_id", json!("wrong-derived-id")),
        ("/hits/hits/0/_source/rom_kind", json!("foreign")),
        ("/hits/hits/0/_source/rom_profile", json!("foreign")),
        ("/hits/hits/0/_source/rom_live", json!(false)),
    ] {
        let mut changed = response();
        *changed.pointer_mut(pointer).unwrap() = value;
        rejected(&changed, 1);
    }
    let mut changed = response();
    changed["hits"]["hits"][0]["_source"]["rom_values"] = json!({"secret":"hidden"});
    rejected(&changed, 1);
}
#[test]
fn invalid_revision_duplicates_and_overrun_fail_closed() {
    for revision in [json!(0), json!(u64::MAX), json!("1")] {
        let mut changed = response();
        changed["hits"]["hits"][0]["_source"]["rom_revision"] = revision.clone();
        changed["hits"]["hits"][0]["_version"] = revision;
        rejected(&changed, 1);
    }
    let mut changed = response();
    changed["hits"]["hits"][0]["_version"] = json!(2);
    rejected(&changed, 1);
    let mut changed = response();
    let duplicate = changed["hits"]["hits"][0].clone();
    changed["hits"]["hits"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    rejected(&changed, 1);
    rejected(&changed, 2);
}
