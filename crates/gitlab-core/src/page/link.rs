use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rel {
    First,
    Prev,
    Next,
    Last,
}

impl Rel {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "first" => Some(Self::First),
            "prev" => Some(Self::Prev),
            "next" => Some(Self::Next),
            "last" => Some(Self::Last),
            _ => None,
        }
    }
}

pub fn parse_link_header(header: &str) -> Result<HashMap<Rel, String>> {
    let mut out = HashMap::new();
    if header.trim().is_empty() {
        return Ok(out);
    }
    for entry in header.split(',') {
        let entry = entry.trim();
        let Some((url_part, rest)) = entry.split_once(';') else { continue; };
        let url = url_part.trim();
        if !url.starts_with('<') || !url.ends_with('>') {
            continue;
        }
        let url = &url[1..url.len() - 1];
        for param in rest.split(';') {
            let param = param.trim();
            let Some((k, v)) = param.split_once('=') else { continue; };
            if k.trim() != "rel" {
                continue;
            }
            let v = v.trim().trim_matches('"');
            if let Some(rel) = Rel::parse(v) {
                out.insert(rel, url.to_owned());
            }
        }
    }
    Ok(out)
}
