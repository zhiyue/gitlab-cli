use async_stream::try_stream;
use futures::Stream;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{GitlabError, Result};
use crate::page::link::{parse_link_header, Rel};

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub path: String,
    pub query: Vec<(String, String)>,
    pub per_page: u32,
}

impl PageRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), query: Vec::new(), per_page: 100 }
    }

    #[must_use]
    pub fn with_query(mut self, pairs: &[(&str, &str)]) -> Self {
        for (k, v) in pairs {
            self.query.push(((*k).to_owned(), (*v).to_owned()));
        }
        self
    }
}

pub struct PagedStream<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned + Send + 'static> PagedStream<T> {
    pub fn start(client: &Client, req: PageRequest) -> impl Stream<Item = Result<T>> + Unpin {
        let client = client.clone();
        let stream = try_stream! {
            let mut url = client.endpoint(&req.path)?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("per_page", &req.per_page.to_string());
                q.append_pair("page", "1");
                for (k, v) in &req.query {
                    q.append_pair(k, v);
                }
            }
            loop {
                let resp = client
                    .http()
                    .get(url.clone())
                    .header("PRIVATE-TOKEN", client.token())
                    .send()
                    .await
                    .map_err(|e| GitlabError::Network(e.to_string()))?;
                let status = resp.status().as_u16();
                let headers = resp.headers().clone();
                let items: Vec<T> = if resp.status().is_success() {
                    resp
                        .json()
                        .await
                        .map_err(|e| GitlabError::Network(format!("parse: {e}")))?
                } else {
                    let msg = resp.text().await.unwrap_or_default();
                    Err(GitlabError::from_status(status, msg, extract_request_id(&headers)))?
                };
                for it in items {
                    yield it;
                }
                let Some(link) = headers.get("link").and_then(|h| h.to_str().ok()) else { break; };
                let rels = parse_link_header(link)?;
                let Some(next) = rels.get(&Rel::Next) else { break; };
                url = url::Url::parse(next)
                    .map_err(|e| GitlabError::Network(format!("bad next url {next}: {e}")))?;
            }
        };
        Box::pin(stream)
    }
}

#[allow(clippy::redundant_closure_for_method_calls)]
fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_owned())
}
