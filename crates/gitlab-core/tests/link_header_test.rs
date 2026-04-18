use gitlab_core::page::link::{parse_link_header, Rel};

#[test]
fn single_next_link() {
    let h = r#"<https://x/api/v4/projects?page=2>; rel="next""#;
    let links = parse_link_header(h).unwrap();
    assert_eq!(
        links.get(&Rel::Next).unwrap(),
        "https://x/api/v4/projects?page=2"
    );
}

#[test]
fn multiple_rels() {
    let h = r#"<https://x/p?page=2>; rel="next", <https://x/p?page=5>; rel="last", <https://x/p?page=1>; rel="first""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.contains_key(&Rel::Next));
    assert!(links.contains_key(&Rel::Last));
    assert!(links.contains_key(&Rel::First));
}

#[test]
fn empty_header_yields_empty_map() {
    let links = parse_link_header("").unwrap();
    assert!(links.is_empty());
}

#[test]
fn malformed_entry_is_skipped() {
    let h = r#"<https://x/p?page=2> rel="next""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.is_empty());
}

#[test]
fn unknown_rel_is_ignored() {
    let h = r#"<https://x/p?page=2>; rel="weird""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.is_empty());
}
