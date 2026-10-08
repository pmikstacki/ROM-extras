//! Controlled native HTTP interruption and object-version race tests.
use rom_azure_blob::{AzureBlob, AzureConfig, Credentials, EndpointPolicy, Limits};
use rom_blob::{BlobStore, Digest, Error, ObjectKey};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
fn adapter(endpoint: &str, deadline: Duration) -> AzureBlob {
    let key = format!("{}==", "A".repeat(86));
    AzureBlob::connect(
        AzureConfig {
            account: "fixtureaccount",
            container: "rom-extras",
            endpoint,
            policy: EndpointPolicy::LoopbackEmulator,
            credentials: Credentials::SharedKey(&key),
        },
        Limits {
            max_bytes: 16,
            max_in_flight: 1,
            deadline,
        },
    )
    .unwrap()
}
fn key() -> ObjectKey {
    ObjectKey::parse(Digest::of(b"false").as_str()).unwrap()
}
async fn headers(stream: &mut TcpStream) -> Vec<u8> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut result = Vec::new();
        loop {
            let byte = stream.read_u8().await.unwrap();
            result.push(byte);
            assert!(result.len() <= 8192, "fixture header bound");
            if result.ends_with(b"\r\n\r\n") {
                return result;
            }
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn busy_admission_refuses_extra_request_and_unacknowledged_write_is_unknown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/fixtureaccount", listener.local_addr().unwrap());
    let adapter = adapter(&endpoint, Duration::from_millis(500));
    let key = key();
    let publication = adapter.create(&key, b"false".to_vec());
    tokio::pin!(publication);
    let (mut socket, _) = tokio::select! { connection=listener.accept()=>connection.unwrap(), result=&mut publication=>panic!("publication completed before request: {result:?}") };
    tokio::select! { _=headers(&mut socket)=>{}, result=&mut publication=>panic!("publication completed before headers: {result:?}") }
    assert_eq!(adapter.head(&key).await, Err(Error::Overloaded));
    assert_eq!(publication.await, Err(Error::Unknown));
    // No fixture acknowledgement was sent; no noncommit claim is made.
}
#[tokio::test]
async fn etag_change_between_metadata_and_body_is_refused() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/fixtureaccount", listener.local_addr().unwrap());
    let adapter = adapter(&endpoint, Duration::from_secs(3));
    let key = key();
    let server = async {
        let (mut head, _) = listener.accept().await.unwrap();
        let _ = headers(&mut head).await;
        head.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nETag: \"first\"\r\nLast-Modified: Thu, 08 Oct 2026 00:00:00 GMT\r\nConnection: close\r\n\r\n").await.unwrap();
        drop(head);
        let (mut get, _) = listener.accept().await.unwrap();
        let request = headers(&mut get).await;
        let text = std::str::from_utf8(&request).unwrap().to_ascii_lowercase();
        let conditional = text
            .lines()
            .any(|line| line.trim() == "if-match: \"first\"");
        let response = if conditional {
            b"HTTP/1.1 412 Precondition Failed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .as_slice()
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nETag: \"second\"\r\nLast-Modified: Thu, 08 Oct 2026 00:00:00 GMT\r\nConnection: close\r\n\r\nother".as_slice()
        };
        get.write_all(response).await.unwrap();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(4), async {
        tokio::join!(adapter.get(&key, 16), server)
    })
    .await
    .unwrap();
    assert_eq!(result, Err(Error::Conflict));
}
