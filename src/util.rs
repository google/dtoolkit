#[must_use]
pub(crate) fn name_without_address(name: &str) -> &str {
    if let Some((name, _)) = name.split_once('@') {
        name
    } else {
        name
    }
}
