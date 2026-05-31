pub const SYNC_PASSWORD_HEADER: &str = "X-Nerevar-Sync-Password";

pub fn sync_password_required(expected: &str) -> bool {
    !expected.is_empty()
}

pub fn sync_password_matches(expected: &str, provided: Option<&str>) -> bool {
    if expected.is_empty() {
        return true;
    }
    provided.map(|value| value == expected).unwrap_or(false)
}
