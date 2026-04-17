use futures::StreamExt;
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::page::{PageRequest, PagedStream};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_3_pages(server: &MockServer) {
    let p1 = serde_json::json!([{"id":1},{"id":2}]);
    let p2 = serde_json::json!([{"id":3},{"id":4}]);
    let p3 = serde_json::json!([{"id":5}]);

    let base = server.uri();
    let link_p1 = format!(
        r#"<{base}/api/v4/projects?page=2&per_page=100>; rel="next", <{base}/api/v4/projects?page=3&per_page=100>; rel="last""#
    );
    let link_p2 = format!(
        r#"<{base}/api/v4/projects?page=3&per_page=100>; rel="next", <{base}/api/v4/projects?page=3&per_page=100>; rel="last""#
    );

    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p1).insert_header("Link", &link_p1))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p2).insert_header("Link", &link_p2))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p3))
        .mount(server)
        .await;
}

#[tokio::test]
async fn streams_across_all_pages() {
    let server = MockServer::start().await;
    setup_3_pages(&server).await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let req = PageRequest::new("projects").with_query(&[("state", "opened")]);
    let stream = PagedStream::<serde_json::Value>::start(&client, req);
    let items: Vec<_> = stream.collect().await;
    assert_eq!(items.len(), 5, "got items: {items:?}");
    for (i, item) in items.into_iter().enumerate() {
        let v = item.unwrap();
        assert_eq!(v["id"], serde_json::json!(i + 1));
    }
}

#[tokio::test]
async fn empty_first_page_yields_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let stream = PagedStream::<serde_json::Value>::start(&client, PageRequest::new("projects"));
    let items: Vec<_> = stream.collect().await;
    assert!(items.is_empty());
}
