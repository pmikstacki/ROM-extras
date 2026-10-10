//! Public private-artifact validation. These inputs are authored metadata, not trusted checkpoint evidence.
use rom_blob::{Digest, ObjectKey};
use rom_blob_checkpoint::{Error, Inventory, Limits};
use serde_json::{Value, json};
fn value() -> Value {
    json!({"version":1,"backend":"Sqlite","checkpoint":Digest::of(b"archive").as_str(),"counts":{"ready":1,"pending":0,"detached":0,"tombstones":0,"other_rows":0},"bindings":[{"id":"Exact/ß:A","revision":2,"upload_revision":1,"owner":"private-owner","store":"private-alias","key":ObjectKey::parse(Digest::of(b"physical-publication").as_str()).unwrap().as_str(),"digest":Digest::of(b"x").as_str(),"bytes":1}]})
}
fn decode(v: Value, limits: Limits) -> Result<Inventory, Error> {
    Inventory::decode(&serde_json::to_vec(&v).unwrap(), limits)
}
#[test]
fn decoded_inventory_limits_distinct_store_aliases() {
    let mut v = value();
    let template = v["bindings"][0].clone();
    v["bindings"] = json!([]);
    for index in 0..33 {
        let mut b = template.clone();
        b["id"] = json!(format!("id-{index}"));
        b["store"] = json!(format!("store-{index}"));
        v["bindings"].as_array_mut().unwrap().push(b);
    }
    v["counts"]["ready"] = json!(33);
    assert_eq!(
        decode(v.clone(), Limits::default()).err(),
        Some(Error::TooLarge)
    );
    v["bindings"].as_array_mut().unwrap().pop();
    v["counts"]["ready"] = json!(32);
    assert!(decode(v, Limits::default()).is_ok());
}
#[test]
fn aliases_partition_identical_physical_keys_without_normalizing_resource_ids() {
    let mut v = value();
    let mut b = v["bindings"][0].clone();
    b["id"] = json!("exact/ß:a");
    b["store"] = json!("another");
    v["bindings"].as_array_mut().unwrap().push(b);
    v["counts"]["ready"] = json!(2);
    let i = decode(v, Limits::default()).unwrap();
    assert_eq!(i.bindings()[0].resource_key().id, "Exact/ß:A");
    assert_eq!(i.bindings()[1].resource_key().id, "exact/ß:a");
    assert_eq!(i.for_store("private-alias").unwrap().entries().len(), 1);
    assert_eq!(i.for_store("another").unwrap().entries().len(), 1);
    assert_eq!(i.for_store("absent").err(), Some(Error::Missing));
    let d = format!("{i:?} {:?}", i.bindings()[0]);
    for text in [
        "private-owner",
        "private-alias",
        "Exact/ß:A",
        i.bindings()[0].entry().key().as_str(),
    ] {
        assert!(!d.contains(text));
    }
}
#[test]
fn duplicate_resource_ids_and_same_store_publications_are_rejected() {
    for change in [false, true] {
        let mut v = value();
        let mut b = v["bindings"][0].clone();
        if change {
            b["id"] = json!("other");
        }
        v["bindings"].as_array_mut().unwrap().push(b);
        v["counts"]["ready"] = json!(2);
        assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    }
}
#[test]
fn malformed_future_and_unknown_fields_have_fixed_errors() {
    let mut v = value();
    v["version"] = json!(2);
    assert_eq!(decode(v, Limits::default()).err(), Some(Error::Unsupported));
    for scope in [0, 1, 2] {
        let mut v = value();
        match scope {
            0 => v["secret"] = json!("never leak"),
            1 => v["bindings"][0]["secret"] = json!("never leak"),
            _ => v["counts"]["secret"] = json!("never leak"),
        };
        assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    }
    assert_eq!(
        Inventory::decode(b"{malformed-secret", Limits::default()).err(),
        Some(Error::Invalid)
    );
    for field in ["key", "digest"] {
        let mut v = value();
        v["bindings"][0][field] = json!("not-an-identity");
        assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    }
}
#[test]
fn revision_identity_and_count_boundaries_are_validated() {
    for (field, n) in [
        ("revision", 0),
        ("revision", u64::MAX),
        ("upload_revision", 0),
        ("upload_revision", 3),
    ] {
        let mut v = value();
        v["bindings"][0][field] = json!(n);
        assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    }
    for field in ["id", "owner", "store"] {
        let mut v = value();
        v["bindings"][0][field] = json!("");
        assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    }
    let mut v = value();
    v["counts"]["ready"] = json!(0);
    assert_eq!(decode(v, Limits::default()).err(), Some(Error::Invalid));
    let mut v = value();
    v["counts"]["other_rows"] = json!(usize::MAX);
    assert_eq!(decode(v, Limits::default()).err(), Some(Error::TooLarge));
    let mut v = value();
    v["bindings"][0]["owner"] = json!("x".repeat(4097));
    assert_eq!(decode(v, Limits::default()).err(), Some(Error::TooLarge));
}
#[test]
fn limits_cover_all_aliases_and_encoded_json() {
    for l in [
        Limits {
            max_ready: 0,
            ..Limits::default()
        },
        Limits {
            max_object_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_json_bytes: 32,
            ..Limits::default()
        },
    ] {
        assert_eq!(decode(value(), l).err(), Some(Error::TooLarge));
    }
    let mut v = value();
    let mut b = v["bindings"][0].clone();
    b["id"] = json!("another");
    b["store"] = json!("another");
    v["bindings"].as_array_mut().unwrap().push(b);
    v["counts"]["ready"] = json!(2);
    assert_eq!(
        decode(
            v,
            Limits {
                max_total_bytes: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::TooLarge)
    );
    assert_eq!(
        decode(
            value(),
            Limits {
                max_json_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Invalid)
    );
    assert_eq!(
        decode(
            value(),
            Limits {
                max_object_bytes: 16 * 1024 * 1024 + 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::TooLarge)
    );
}
#[test]
#[cfg(unix)]
fn durable_artifact_refuses_overwrite_public_permissions_and_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = std::env::temp_dir().join(format!(
        "rom-extras-inventory-file-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let i = decode(value(), Limits::default()).unwrap();
    let file = root.join("private.json");
    i.publish(&file).unwrap();
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let original = std::fs::read(&file).unwrap();
    assert_eq!(i.publish(&file), Err(Error::Conflict));
    assert_eq!(std::fs::read(&file).unwrap(), original);
    assert_eq!(
        Inventory::read(&file, Limits::default())
            .unwrap()
            .encode()
            .unwrap(),
        i.encode().unwrap()
    );
    assert_eq!(
        Inventory::read(
            &file,
            Limits {
                max_json_bytes: 16,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::TooLarge)
    );
    let link = root.join("link");
    symlink(&file, &link).unwrap();
    assert_eq!(
        Inventory::read(&link, Limits::default()).err(),
        Some(Error::Invalid)
    );
    assert_eq!(i.publish(&link), Err(Error::Conflict));
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        Inventory::read(&file, Limits::default()).err(),
        Some(Error::Invalid)
    );
}
#[test]
#[cfg(not(feature = "sqlite"))]
fn disabled_sqlite_fails_before_path_or_store_admission() {
    assert_eq!(
        rom_blob_checkpoint::Inspection::prepare(
            rom_backup::Backend::Sqlite,
            std::path::Path::new("missing-private-archive"),
            std::path::Path::new("missing-inspection"),
            Vec::new(),
            Limits::default()
        )
        .err(),
        Some(Error::Unsupported)
    );
}
#[test]
#[cfg(not(feature = "redb"))]
fn disabled_redb_fails_before_path_or_store_admission() {
    assert_eq!(
        rom_blob_checkpoint::Inspection::prepare(
            rom_backup::Backend::Redb,
            std::path::Path::new("missing-private-archive"),
            std::path::Path::new("missing-inspection"),
            Vec::new(),
            Limits::default()
        )
        .err(),
        Some(Error::Unsupported)
    );
}
