use reqwest::Method;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub method: Method,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

impl RequestSpec {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            body: None,
        }
    }

    #[must_use]
    pub fn with_query<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in pairs {
            self.query.push((k.into(), v.into()));
        }
        self
    }

    #[must_use]
    pub fn with_json<T: Serialize>(mut self, body: &T) -> Self {
        self.body = Some(serde_json::to_value(body).unwrap_or(serde_json::Value::Null));
        self
    }
}
