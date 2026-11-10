//! API 13's additive download method does not make caching or tickets mandatory.
use rexrs::api::{
    BlockMethod,
    terminal::{DownloadBegin, DownloadBeginResponse},
};
use serde_json::{Value, json};

#[test]
fn download_request_preserves_paths_and_optional_cache_validator() {
    let request = DownloadBegin::new("download:00000000-0000-4000-8000-000000000001", "~/a file");
    assert_eq!(
        DownloadBegin::NAME,
        "com.superlogical.terminal.download_begin"
    );
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["ref"], request.r#ref);
    assert_eq!(value["path"], "~/a file");
    assert!(value.get("if_none_match").is_none());
    let value = serde_json::to_value(request.if_none_match("opaque-version")).unwrap();
    assert_eq!(value["if_none_match"], "opaque-version");
}

fn metadata() -> Value {
    json!({"path":"/private/a file", "name":"a file", "size":42,
           "modified_at":"2026-11-10T07:32:00-08:00"})
}

#[test]
fn download_ticket_and_uncacheable_response_roundtrip() {
    let mut value = metadata();
    value["ticket"] = json!("single-use");
    value["expires_in_ms"] = json!(5000);
    let response: DownloadBeginResponse = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(response.ticket.as_deref(), Some("single-use"));
    assert_eq!(response.expires_in_ms, Some(5000));
    assert_eq!(response.not_modified, None);
    assert_eq!(response.version, None);
    assert_eq!(serde_json::to_value(response).unwrap(), value);
}

#[test]
fn unchanged_download_has_no_ticket() {
    let mut value = metadata();
    value["not_modified"] = json!(true);
    value["version"] = json!("opaque-version");
    let response: DownloadBeginResponse = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(response.not_modified, Some(true));
    assert_eq!(response.ticket, None);
    assert_eq!(response.expires_in_ms, None);
    assert_eq!(response.version.as_deref(), Some("opaque-version"));
    assert_eq!(serde_json::to_value(response).unwrap(), value);
}

#[test]
fn download_metadata_is_required() {
    for key in ["path", "name", "size", "modified_at"] {
        let mut value = metadata();
        value.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<DownloadBeginResponse>(value).is_err(),
            "{key}"
        );
    }
}
