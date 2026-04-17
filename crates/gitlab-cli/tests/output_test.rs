// integration test lives in the bin crate; we use lib target through inline mod.
// Instead, test via process output once a command exists (deferred to version/me tasks).
#[test]
fn placeholder_until_commands_exist() {
    assert!(true);
}
