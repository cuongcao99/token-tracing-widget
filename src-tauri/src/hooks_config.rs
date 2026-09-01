use serde_json::{Map, Value};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookProvider {
    Claude,
    Codex,
}

impl HookProvider {
    fn events(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &["UserPromptSubmit", "Stop", "StopFailure", "SessionEnd"],
            Self::Codex => &["UserPromptSubmit", "Stop", "SessionEnd"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookConfigError {
    EmptyCommand,
    RootMustBeObject,
    InvalidShape {
        path: String,
        expected: &'static str,
    },
}

impl fmt::Display for HookConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommand => formatter.write_str("hook command must not be empty"),
            Self::RootMustBeObject => formatter.write_str("hook config root must be an object"),
            Self::InvalidShape { path, expected } => {
                write!(formatter, "hook config field {path} must be {expected}")
            }
        }
    }
}

impl Error for HookConfigError {}

pub fn merge_provider_hooks(
    mut config: Value,
    provider: HookProvider,
    command: &str,
) -> Result<Value, HookConfigError> {
    validate_command(command)?;
    validate_config(&config)?;

    let root = config
        .as_object_mut()
        .expect("config was validated as an object");
    let hooks_value = root
        .entry("hooks".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks_value
        .as_object_mut()
        .expect("hooks was validated as an object or just created");

    for event in provider.events() {
        let groups_value = hooks
            .entry((*event).to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
        let groups = groups_value
            .as_array_mut()
            .expect("event groups were validated as an array or just created");

        if groups
            .iter()
            .any(|group| group_contains_owned_handler(group, provider, command))
        {
            continue;
        }

        let mut group = Map::new();
        group.insert(
            "hooks".to_owned(),
            Value::Array(vec![generated_handler(provider, command)]),
        );
        groups.push(Value::Object(group));
    }

    Ok(config)
}

pub fn remove_provider_hooks(
    mut config: Value,
    provider: HookProvider,
    command: &str,
) -> Result<Value, HookConfigError> {
    validate_command(command)?;
    validate_config(&config)?;

    let root = config
        .as_object_mut()
        .expect("config was validated as an object");
    let Some(hooks_value) = root.get_mut("hooks") else {
        return Ok(config);
    };
    let hooks = hooks_value
        .as_object_mut()
        .expect("hooks was validated as an object");

    for event in provider.events() {
        let remove_event = {
            let Some(groups_value) = hooks.get_mut(*event) else {
                continue;
            };
            let groups = groups_value
                .as_array_mut()
                .expect("event groups were validated as an array");
            let mut retained = Vec::with_capacity(groups.len());
            let mut removed_app_group = false;

            for mut group in groups.drain(..) {
                let group_object = group
                    .as_object_mut()
                    .expect("matcher groups were validated as objects");
                let handlers = group_object
                    .get_mut("hooks")
                    .expect("matcher groups were validated with hooks")
                    .as_array_mut()
                    .expect("group hooks were validated as an array");
                let before = handlers.len();
                handlers.retain(|handler| !is_owned_handler(handler, provider, command));
                let removed_handler = handlers.len() != before;

                if removed_handler && handlers.is_empty() && group_object.len() == 1 {
                    removed_app_group = true;
                    continue;
                }

                retained.push(group);
            }

            let event_is_empty = retained.is_empty() && removed_app_group;
            *groups = retained;
            event_is_empty
        };

        if remove_event {
            hooks.remove(*event);
        }
    }

    if hooks.is_empty() {
        root.remove("hooks");
    }

    Ok(config)
}

fn validate_command(command: &str) -> Result<(), HookConfigError> {
    if command.trim().is_empty() {
        Err(HookConfigError::EmptyCommand)
    } else {
        Ok(())
    }
}

fn validate_config(config: &Value) -> Result<(), HookConfigError> {
    let root = config
        .as_object()
        .ok_or(HookConfigError::RootMustBeObject)?;
    let Some(hooks_value) = root.get("hooks") else {
        return Ok(());
    };
    let hooks = hooks_value
        .as_object()
        .ok_or_else(|| invalid_shape("hooks", "an object"))?;

    for (event, groups_value) in hooks {
        let groups = groups_value
            .as_array()
            .ok_or_else(|| invalid_shape(format!("hooks.{event}"), "an array"))?;

        for (group_index, group_value) in groups.iter().enumerate() {
            let path = format!("hooks.{event}[{group_index}]");
            let group = group_value
                .as_object()
                .ok_or_else(|| invalid_shape(&path, "an object"))?;

            if let Some(matcher) = group.get("matcher") {
                if !matcher.is_string() {
                    return Err(invalid_shape(format!("{path}.matcher"), "a string"));
                }
            }

            let handlers_value = group
                .get("hooks")
                .ok_or_else(|| invalid_shape(format!("{path}.hooks"), "an array"))?;
            let handlers = handlers_value
                .as_array()
                .ok_or_else(|| invalid_shape(format!("{path}.hooks"), "an array"))?;

            for (handler_index, handler) in handlers.iter().enumerate() {
                if !handler.is_object() {
                    return Err(invalid_shape(
                        format!("{path}.hooks[{handler_index}]"),
                        "an object",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn invalid_shape(path: impl Into<String>, expected: &'static str) -> HookConfigError {
    HookConfigError::InvalidShape {
        path: path.into(),
        expected,
    }
}

fn generated_handler(provider: HookProvider, command: &str) -> Value {
    let mut handler = Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command.to_owned()));

    if provider == HookProvider::Codex {
        handler.insert(
            "commandWindows".to_owned(),
            Value::String(command.to_owned()),
        );
    }

    Value::Object(handler)
}

fn group_contains_owned_handler(group: &Value, provider: HookProvider, command: &str) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .map(|handlers| {
            handlers
                .iter()
                .any(|handler| is_owned_handler(handler, provider, command))
        })
        .unwrap_or(false)
}

fn is_owned_handler(handler: &Value, provider: HookProvider, command: &str) -> bool {
    let Some(handler) = handler.as_object() else {
        return false;
    };

    if handler.get("type").and_then(Value::as_str) != Some("command")
        || handler.get("command").and_then(Value::as_str) != Some(command)
    {
        return false;
    }

    match provider {
        HookProvider::Claude => !handler.contains_key("commandWindows"),
        HookProvider::Codex => {
            handler.get("commandWindows").and_then(Value::as_str) == Some(command)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_provider_hooks, remove_provider_hooks, HookProvider};
    use serde_json::{json, Value};

    const COMMAND: &str = "TokenTracingWidget.exe --hook";

    fn hook_events(provider: HookProvider) -> &'static [&'static str] {
        match provider {
            HookProvider::Claude => &["UserPromptSubmit", "Stop", "StopFailure", "SessionEnd"],
            HookProvider::Codex => &["UserPromptSubmit", "Stop", "SessionEnd"],
        }
    }

    fn generated_hook<'a>(config: &'a Value, event: &str) -> &'a Value {
        &config["hooks"][event][0]["hooks"][0]
    }

    #[test]
    fn merge_empty_claude_config_creates_only_supported_events() {
        let result = merge_provider_hooks(json!({}), HookProvider::Claude, COMMAND).unwrap();
        let hooks = result["hooks"].as_object().unwrap();

        assert_eq!(hooks.len(), 4);
        for event in hook_events(HookProvider::Claude) {
            assert_eq!(result["hooks"][*event].as_array().unwrap().len(), 1);
            assert_eq!(
                generated_hook(&result, event),
                &json!({"type": "command", "command": COMMAND})
            );
            assert!(generated_hook(&result, event)
                .get("commandWindows")
                .is_none());
        }
    }

    #[test]
    fn merge_empty_codex_config_serializes_windows_command_alongside_command() {
        let result = merge_provider_hooks(json!({}), HookProvider::Codex, COMMAND).unwrap();
        let hooks = result["hooks"].as_object().unwrap();

        assert_eq!(hooks.len(), 3);
        for event in hook_events(HookProvider::Codex) {
            assert_eq!(
                generated_hook(&result, event),
                &json!({
                    "type": "command",
                    "command": COMMAND,
                    "commandWindows": COMMAND
                })
            );
        }
        assert!(result["hooks"].get("StopFailure").is_none());
    }

    #[test]
    fn merge_preserves_unrelated_keys_and_hooks() {
        let existing = json!({
            "description": "keep this",
            "custom": {"private": "unrelated configuration"},
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{"type": "command", "command": "keep-pre-tool"}]
                }],
                "UserPromptSubmit": [{
                    "matcher": "provider-owned",
                    "hooks": [{"type": "command", "command": "keep-prompt"}]
                }]
            }
        });

        let result = merge_provider_hooks(existing.clone(), HookProvider::Claude, COMMAND).unwrap();

        assert_eq!(result["description"], existing["description"]);
        assert_eq!(result["custom"], existing["custom"]);
        assert_eq!(
            result["hooks"]["PreToolUse"],
            existing["hooks"]["PreToolUse"]
        );
        assert_eq!(
            result["hooks"]["UserPromptSubmit"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            result["hooks"]["UserPromptSubmit"][0],
            existing["hooks"]["UserPromptSubmit"][0]
        );
        assert_eq!(
            result["hooks"]["UserPromptSubmit"][1]["hooks"][0],
            json!({
                "type": "command",
                "command": COMMAND
            })
        );
    }

    #[test]
    fn merge_is_idempotent_for_both_providers() {
        for provider in [HookProvider::Claude, HookProvider::Codex] {
            let once = merge_provider_hooks(json!({}), provider, COMMAND).unwrap();
            let twice = merge_provider_hooks(once.clone(), provider, COMMAND).unwrap();

            assert_eq!(twice, once);
        }
    }

    #[test]
    fn remove_round_trip_removes_only_generated_config() {
        for provider in [HookProvider::Claude, HookProvider::Codex] {
            let installed = merge_provider_hooks(json!({}), provider, COMMAND).unwrap();
            let removed = remove_provider_hooks(installed, provider, COMMAND).unwrap();

            assert_eq!(removed, json!({}));
        }
    }

    #[test]
    fn remove_preserves_other_hooks_in_the_same_group() {
        let existing = json!({
            "hooks": {
                "Stop": [{
                    "hooks": [
                        {"type": "command", "command": "keep-this-command"},
                        {"type": "command", "command": COMMAND}
                    ]
                }],
                "OtherEvent": [{"hooks": [{"type": "command", "command": "keep-other"}]}]
            }
        });

        let result = remove_provider_hooks(existing, HookProvider::Claude, COMMAND).unwrap();

        assert_eq!(
            result["hooks"]["Stop"][0]["hooks"],
            json!([{"type": "command", "command": "keep-this-command"}])
        );
        assert_eq!(
            result["hooks"]["OtherEvent"],
            json!([{"hooks": [{"type": "command", "command": "keep-other"}]}])
        );
    }

    #[test]
    fn remove_does_not_delete_lookalike_handlers() {
        let existing = json!({
            "hooks": {
                "UserPromptSubmit": [{
                    "hooks": [
                        {"type": "http", "command": COMMAND},
                        {"type": "command", "command": "another-command"},
                        {
                            "type": "command",
                            "command": COMMAND,
                            "commandWindows": "different-windows-command"
                        }
                    ]
                }]
            }
        });

        let result = remove_provider_hooks(existing, HookProvider::Codex, COMMAND).unwrap();
        assert_eq!(
            result["hooks"]["UserPromptSubmit"][0]["hooks"],
            json!([
                {"type": "http", "command": COMMAND},
                {"type": "command", "command": "another-command"},
                {
                    "type": "command",
                    "command": COMMAND,
                    "commandWindows": "different-windows-command"
                }
            ])
        );
    }

    #[test]
    fn malformed_config_is_rejected_without_mutating_the_input() {
        let cases = [
            (Value::String("not an object".to_owned()), "root"),
            (json!({"hooks": []}), "hooks"),
            (json!({"hooks": {"Stop": {}}}), "event"),
            (json!({"hooks": {"Stop": [{"hooks": {}}]}}), "group hooks"),
            (
                json!({"hooks": {"Stop": [{"hooks": ["not a handler"]}]}}),
                "handler",
            ),
        ];

        for (input, label) in cases {
            let snapshot = input.clone();
            assert!(
                merge_provider_hooks(input.clone(), HookProvider::Claude, COMMAND).is_err(),
                "malformed {label} config should be rejected"
            );
            assert_eq!(input, snapshot, "invalid input must remain unchanged");
        }
    }

    #[test]
    fn malformed_config_is_rejected_for_removal_too() {
        let input = json!({"hooks": {"SessionEnd": "corrupt"}});

        assert!(remove_provider_hooks(input, HookProvider::Codex, COMMAND).is_err());
    }

    #[test]
    fn generated_config_contains_no_raw_hook_payload_fields_or_values() {
        let result = merge_provider_hooks(json!({}), HookProvider::Codex, COMMAND).unwrap();
        let serialized = serde_json::to_string(&result).unwrap();

        for forbidden in [
            "private prompt must never enter config",
            "C:/private/transcript.jsonl",
            "private assistant message",
            "private working directory",
            "raw input token count",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "found forbidden value: {forbidden}"
            );
        }

        for event in hook_events(HookProvider::Codex) {
            let handler = generated_hook(&result, event).as_object().unwrap();
            assert_eq!(handler.len(), 3);
            assert!(handler.get("prompt").is_none());
            assert!(handler.get("transcriptPath").is_none());
            assert!(handler.get("cwd").is_none());
        }
    }
}
