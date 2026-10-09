//! Test host administrative client is separate from collection-scoped writer credentials.
use reqwest::{Client, Method};
use rom::{JournalView, Key, ProjectedView, json};
use rom_projection_core::{
    ApprovedDocument, DocumentMapping, ProjectionProfile, ProjectionTarget, TargetFailure,
};
use rom_qdrant::{Distance, Generation, PreparedWrite, Qdrant, TlsConfig};
use serde_json::Value;
use std::{fs, time::Duration};
pub struct Host {
    config: Value,
    mapping: DocumentMapping,
    admin: Client,
}
impl Host {
    pub fn load(path: &str) -> Self {
        let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let ca = fs::read(config["ca"].as_str().unwrap()).unwrap();
        let mut token =
            reqwest::header::HeaderValue::from_str(config["admin"].as_str().unwrap()).unwrap();
        token.set_sensitive(true);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("api-key", token);
        let admin = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .default_headers(headers)
            .add_root_certificate(reqwest::Certificate::from_pem(&ca).unwrap())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let mapping = Self::mapping_definition();
        Self {
            config,
            mapping,
            admin,
        }
    }
    fn generation(&self, i: usize) -> Generation {
        let row = &self.config["generations"][i];
        Generation::new(
            self.mapping.profile().clone(),
            row["physical"].as_str().unwrap(),
            row["nonce"].as_str().unwrap(),
            3,
            [Distance::Dot, Distance::Euclid, Distance::Manhattan][i],
        )
        .unwrap()
    }
    fn mapping_definition() -> DocumentMapping {
        DocumentMapping::new(
            ProjectionProfile::new(
                "native-consumer",
                "qdrant",
                "mapping-v1",
                Some("host-model-v1"),
            )
            .unwrap(),
            vec!["value".into()],
            Some(3),
        )
        .unwrap()
    }
    pub(crate) fn mapping(&self) -> DocumentMapping {
        Self::mapping_definition()
    }
    pub(crate) fn target(&self, i: usize) -> Qdrant {
        self.target_credential(i, "writer")
    }
    pub(crate) fn reader(&self, i: usize) -> Qdrant {
        self.target_credential(i, "reader")
    }
    fn target_credential(&self, i: usize, credential: &str) -> Qdrant {
        let row = &self.config["generations"][i];
        Qdrant::new(
            TlsConfig::api_key(
                self.config["endpoint"].as_str().unwrap(),
                fs::read(self.config["ca"].as_str().unwrap()).unwrap(),
                row[credential].as_str().unwrap().as_bytes().to_vec(),
                Duration::from_secs(5),
            )
            .unwrap(),
            self.generation(i),
        )
        .unwrap()
    }
    pub(crate) async fn control(&self, method: Method, path: &str, body: Value) -> Value {
        let response = self
            .admin
            .request(
                method,
                format!("{}{}", self.config["endpoint"].as_str().unwrap(), path),
            )
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&body).unwrap())
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        serde_json::from_slice(&response.bytes().await.unwrap()).unwrap()
    }
    pub fn definitions(&self) -> Value {
        json!(
            (0..3)
                .map(
                    |i| serde_json::from_slice::<Value>(&self.generation(i).definition().unwrap())
                        .unwrap()
                )
                .collect::<Vec<_>>()
        )
    }
    fn doc(&self, id: &str, rev: u64, live: bool, vector: Vec<f32>) -> ApprovedDocument {
        self.mapping
            .document(
                &JournalView {
                    position: 1,
                    view: ProjectedView {
                        key: Key {
                            kind: "documents".into(),
                            id: id.into(),
                        },
                        revision: rev,
                        value: live.then(|| json!({"value":u64::MAX}).as_object().unwrap().clone()),
                    },
                },
                live.then_some(vector),
            )
            .unwrap()
    }
    pub(crate) async fn admin_call(&self, method: Method, path: &str, body: Value) -> Value {
        let response = self
            .admin
            .request(
                method,
                format!("{}{path}", self.config["endpoint"].as_str().unwrap()),
            )
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&body).unwrap())
            .send()
            .await
            .expect("native host request failed");
        assert!(
            response.status().is_success(),
            "native host status {}",
            response.status()
        );
        let bytes = response.bytes().await.expect("native host response failed");
        assert!(bytes.len() <= 1048576);
        serde_json::from_slice(&bytes).expect("native host invalid response")
    }
    pub async fn refusal(&self, cancel: bool) {
        let mut target = self.target(0);
        let document = self.doc("refusal", 1, true, vec![3., 4., 0.]);
        let prepared = target.prepare(&[&document]).unwrap();
        if cancel {
            assert!(
                tokio::time::timeout(Duration::from_millis(30), target.apply(prepared))
                    .await
                    .is_err()
            );
        } else {
            assert_eq!(
                target.apply(prepared).await.err(),
                Some(TargetFailure::Unknown)
            );
        }
    }
    pub async fn run(&self) {
        for i in 0..3 {
            let mut target = self.target(i);
            target
                .verify_generation()
                .await
                .expect("native generation rejected");
            for (n, vector) in [
                vec![3., 4., 0.],
                vec![f32::MAX, 0., 0.],
                vec![f32::from_bits(1), 1e-38, -0.],
                vec![0., 0., 0.],
            ]
            .into_iter()
            .enumerate()
            {
                let doc = self.doc(&format!("original/{n}"), 1, true, vector);
                let page = target.prepare(&[&doc]).unwrap();
                assert_eq!(target.apply(page).await.unwrap().len(), 1);
                assert_eq!(
                    target
                        .apply(target.prepare(&[&doc]).unwrap())
                        .await
                        .unwrap()
                        .len(),
                    1
                );
                assert_eq!(
                    target
                        .inspect_prepared(&target.prepare(&[&doc]).unwrap())
                        .await
                        .unwrap()
                        .len(),
                    1
                );
            }
            for rev in [1, 1 << 32, 1 << 63, u64::MAX] {
                let doc = self.doc("full/u64", rev, true, vec![3., 4., 0.]);
                assert_eq!(
                    target
                        .apply(target.prepare(&[&doc]).unwrap())
                        .await
                        .unwrap()
                        .len(),
                    1
                );
            }
            let older = self.doc("full/u64", 1, true, vec![3., 4., 0.]);
            assert_eq!(
                target.apply(target.prepare(&[&older]).unwrap()).await.err(),
                Some(TargetFailure::Rejected)
            );
            let conflict = self.doc("full/u64", u64::MAX, true, vec![4., 3., 0.]);
            assert_eq!(
                target
                    .apply(target.prepare(&[&conflict]).unwrap())
                    .await
                    .err(),
                Some(TargetFailure::Rejected)
            );
            let live = self.doc("deleted", 1, true, vec![3., 4., 0.]);
            target
                .apply(target.prepare(&[&live]).unwrap())
                .await
                .unwrap();
            let tomb = self.doc("deleted", 2, false, vec![]);
            target
                .apply(target.prepare(&[&tomb]).unwrap())
                .await
                .unwrap();
            assert_eq!(
                target.apply(target.prepare(&[&live]).unwrap()).await.err(),
                Some(TargetFailure::Rejected)
            );
            let resurrect = self.doc("deleted", 3, true, vec![3., 4., 0.]);
            target
                .apply(target.prepare(&[&resurrect]).unwrap())
                .await
                .unwrap();
            let doc = self.doc("corruption", 1, true, vec![3., 4., 0.]);
            target
                .apply(target.prepare(&[&doc]).unwrap())
                .await
                .unwrap();
            let mut point: Value =
                serde_json::from_slice(PreparedWrite::new(&doc).unwrap().as_bytes()).unwrap();
            let path = format!(
                "collections/{}/points?wait=true&ordering=strong",
                target.physical_target()
            );
            point["points"][0]["vector"]["embedding"] = json!([0.6, 0.8, 0.]);
            self.admin_call(Method::PUT, &path, json!({"points":point["points"]}))
                .await;
            assert_eq!(
                target
                    .inspect_prepared(&target.prepare(&[&doc]).unwrap())
                    .await
                    .err(),
                Some(TargetFailure::Rejected)
            );
            point["points"][0]["vector"]["embedding"] = json!([3., 4., 0.]);
            point["points"][0]["payload"]["rom_id"] = json!("other-key");
            self.admin_call(Method::PUT, &path, json!({"points":point["points"]}))
                .await;
            let newer = self.doc("corruption", 2, true, vec![3., 4., 0.]);
            assert_eq!(
                target.apply(target.prepare(&[&newer]).unwrap()).await.err(),
                Some(TargetFailure::Rejected)
            );
            let missing = self.doc("missing", 1, true, vec![3., 4., 0.]);
            assert_eq!(
                target
                    .inspect_prepared(&target.prepare(&[&missing]).unwrap())
                    .await
                    .err(),
                Some(TargetFailure::Rejected)
            );
            let other = self.target((i + 1) % 3);
            assert_eq!(
                other
                    .inspect_prepared(&target.prepare(&[&missing]).unwrap())
                    .await
                    .err(),
                Some(TargetFailure::Rejected)
            );
            self.admin_call(
                Method::PATCH,
                &format!("collections/{}", target.physical_target()),
                json!({"metadata":{"rom_extras_projection":{"generation":"changed"}}}),
            )
            .await;
            assert_eq!(
                target.verify_generation().await.err(),
                Some(TargetFailure::Rejected)
            );
        }
    }
}
