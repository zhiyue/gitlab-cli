use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{GitlabError, Result};
use crate::page::link::{parse_link_header, Rel};
use crate::request::RequestSpec;

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub path: String,
    pub query: Vec<(String, String)>,
    pub per_page: u32,
}

impl PageRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            query: Vec::new(),
            per_page: 100,
        }
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
    _m: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned + Send + 'static> PagedStream<T> {
    #[allow(
        clippy::while_let_loop,
        clippy::option_if_let_else,
        clippy::manual_let_else
    )]
    pub fn start(client: &Client, req: PageRequest) -> impl Stream<Item = Result<T>> + Unpin {
        let client = client.clone();
        let stream = try_stream! {
            let mut next_path: Option<String> = Some(req.path.clone());
            let mut first = true;
            loop {
                let path = match next_path.take() { Some(p) => p, None => break };
                let mut spec = RequestSpec::new(Method::GET, path);
                if first {
                    spec = spec.with_query(req.query.iter().map(|(k,v)| (k.clone(), v.clone())));
                    spec.query.push(("per_page".to_owned(), req.per_page.to_string()));
                    spec.query.push(("page".to_owned(), "1".to_owned()));
                    first = false;
                }
                let (_status, headers, bytes) = client.send_raw(spec).await?;
                let items: Vec<T> = serde_json::from_slice(&bytes)
                    .map_err(|e| GitlabError::Network(format!("parse: {e}")))?;
                for it in items { yield it; }
                let link = headers.get("link").and_then(|h| h.to_str().ok());
                if let Some(h) = link {
                    let rels = parse_link_header(h)?;
                    if let Some(next) = rels.get(&Rel::Next) {
                        let url = url::Url::parse(next)
                            .map_err(|e| GitlabError::Network(format!("bad next url {next}: {e}")))?;
                        let path_qs = format!("{}?{}", url.path(), url.query().unwrap_or(""));
                        let trimmed = path_qs
                            .trim_start_matches("/api/v4/")
                            .trim_start_matches("api/v4/")
                            .to_owned();
                        next_path = Some(trimmed);
                    }
                }
            }
        };
        Box::pin(stream)
    }
}
