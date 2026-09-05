//! Safe update metadata crossing the Rust/React boundary.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub available_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::UpdateCheckResult;

    #[test]
    fn result_contains_only_safe_version_metadata() {
        let value = serde_json::to_value(UpdateCheckResult {
            current_version: "0.1.0".to_owned(),
            available_version: Some("0.2.0".to_owned()),
        })
        .unwrap();
        let object = value.as_object().unwrap();

        assert_eq!(
            object.keys().map(String::as_str).collect::<Vec<_>>(),
            ["availableVersion", "currentVersion"]
        );
        assert!(!value.to_string().contains("signature"));
        assert!(!value.to_string().contains("url"));
    }
}
