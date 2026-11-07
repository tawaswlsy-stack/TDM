//! #124: the update check asks one list of releases, reads at most 1 MiB of
//! it, and says the newest one newer than this TDM, with its page built
//! on TDM's own releases address. Against the local test server only:
//! never the internet.

use tdm_core::update::{CheckError, newer_release_from};
use tdm_core::{Engine, Options, Stop};
use tdm_testserver::{Failure, FileSpec, TestServer};

fn list(tags: &[&str]) -> Vec<u8> {
    let items: Vec<String> = tags
        .iter()
        .map(|t| format!(r#"{{"tag_name":"{t}","draft":false}}"#))
        .collect();
    format!("[{}]", items.join(",")).into_bytes()
}

#[tokio::test]
async fn a_newer_release_is_said_with_its_own_page() {
    let server = TestServer::http();
    server.put("/releases", FileSpec::new(list(&["v0.1.0", "v0.2.0"])));
    let engine = Engine::new(Options::default()).unwrap();
    let url = server.url("/releases");
    let found = newer_release_from(&engine, &url, "0.1.0", &Stop::new())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.version, "0.2.0");
    assert_eq!(
        found.page,
        "https://github.com/tawaswlsy-stack/TDM/releases/tag/v0.2.0"
    );
    // Up to date: nothing.
    let none = newer_release_from(&engine, &url, "0.2.0", &Stop::new()).await;
    assert!(matches!(none, Ok(None)), "{none:?}");
    assert_eq!(server.gets("/releases").len(), 2, "one request each");
}

#[tokio::test]
async fn a_transient_github_failure_is_retried() {
    let server = TestServer::http();
    server.put(
        "/releases",
        FileSpec::new(list(&["v0.2.0"])).fail_first(vec![Failure {
            status: 503,
            retry_after: None,
        }]),
    );
    let engine = Engine::new(Options::default()).unwrap();

    let found = newer_release_from(&engine, &server.url("/releases"), "0.1.0", &Stop::new())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.version, "0.2.0");
    let gets = server.gets("/releases");
    assert_eq!(
        gets.iter()
            .map(|request| request.status)
            .collect::<Vec<_>>(),
        [503, 200]
    );
}

#[tokio::test]
async fn a_bad_answer_is_an_error_not_a_release() {
    let server = TestServer::http();
    server.put("/huge", FileSpec::new(vec![b' '; 1024 * 1024 + 1]));
    let engine = Engine::new(Options {
        max_failures: 1,
        ..Options::default()
    })
    .unwrap();
    let stop = Stop::new();
    let huge = newer_release_from(&engine, &server.url("/huge"), "0.1.0", &stop).await;
    assert!(matches!(huge, Err(CheckError::Engine(_))), "{huge:?}");
    let missing = newer_release_from(&engine, &server.url("/none"), "0.1.0", &stop).await;
    assert!(matches!(missing, Err(CheckError::Engine(_))), "{missing:?}");
    let version = newer_release_from(&engine, &server.url("/none"), "dev", &stop).await;
    assert!(
        matches!(version, Err(CheckError::Version(_))),
        "{version:?}"
    );
}
