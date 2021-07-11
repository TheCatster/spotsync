use super::*;

#[test]
fn read_refresh_token() {
    let _file = std::fs::read_to_string(".refresh_token");
}

#[test]
fn get_authenticated_user() {
    let _client = authenticate_spotify();
}
