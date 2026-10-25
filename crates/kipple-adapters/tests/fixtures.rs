//! Golden tests: scanning each adapter's fixture with only that integration
//! enabled produces a stable list of items (SPEC §18).

mod common;

macro_rules! fixture_tests {
    ($($name:ident => $id:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                insta::assert_json_snapshot!($id, common::scan_fixture($id));
            }
        )*
    };
}

fixture_tests! {
    claude_code => "claude-code",
    codex => "codex",
}
