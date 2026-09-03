# Native Windows Provider Formats

## Claude Code

### Outcome

| Field | Value |
| --- | --- |
| Outcome | detected |

### Coverage

| Files | Complete records | Byte limit | Record limit | Supported shape |
| ---: | ---: | --- | --- | --- |
| 1 | 74 | false | false | true |

### Layout patterns

- <segment>/<file>.jsonl

### Record shapes

| Discriminator | Counters | Sampled records |
| --- | --- | ---: |
| message | $.message.usage.cache_creation.ephemeral_1h_input_tokens, $.message.usage.cache_creation.ephemeral_5m_input_tokens, $.message.usage.cache_creation_input_tokens, $.message.usage.cache_read_input_tokens, $.message.usage.input_tokens, $.message.usage.output_tokens, $.message.usage.output_tokens_details.thinking_tokens | 30 |

### Counter behavior

| Field | Behavior | Synthetic sequence |
| --- | --- | --- |
| $.message.usage.cache_creation.ephemeral_1h_input_tokens | monotonic | 100, 150, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250 |
| $.message.usage.cache_creation.ephemeral_5m_input_tokens | monotonic | 100, 150, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250 |
| $.message.usage.cache_creation_input_tokens | monotonic | 100, 150, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250 |
| $.message.usage.cache_read_input_tokens | monotonic | 100, 150, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250 |
| $.message.usage.input_tokens | reset_observed | 100, 150, 20, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45 |
| $.message.usage.output_tokens | per_event | 10, 20, 30, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40 |
| $.message.usage.output_tokens_details.thinking_tokens | monotonic | 100, 150, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250 |

### Identity and timestamp paths

- Session: $.sessionId; Event: $.message.id; Timestamp: $.timestamp

### Diagnostics

None observed

### Artifacts

- Manifest: claude/manifest.json
- Records: claude/records.jsonl

### Privacy validation

Validated before write.
## Codex

### Outcome

| Field | Value |
| --- | --- |
| Outcome | detected |

### Coverage

| Files | Complete records | Byte limit | Record limit | Supported shape |
| ---: | ---: | --- | --- | --- |
| 1 | 64 | false | false | true |

### Layout patterns

- <number>/<number>/<number>/<file>.jsonl

### Record shapes

| Discriminator | Counters | Sampled records |
| --- | --- | ---: |
| token_count | $.payload.info.last_token_usage.cache_write_input_tokens, $.payload.info.last_token_usage.cached_input_tokens, $.payload.info.last_token_usage.input_tokens, $.payload.info.last_token_usage.output_tokens, $.payload.info.last_token_usage.reasoning_output_tokens, $.payload.info.last_token_usage.total_tokens, $.payload.info.total_token_usage.cache_write_input_tokens, $.payload.info.total_token_usage.cached_input_tokens, $.payload.info.total_token_usage.input_tokens, $.payload.info.total_token_usage.output_tokens, $.payload.info.total_token_usage.reasoning_output_tokens, $.payload.info.total_token_usage.total_tokens | 6 |

### Counter behavior

| Field | Behavior | Synthetic sequence |
| --- | --- | --- |
| $.payload.info.last_token_usage.cache_write_input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.last_token_usage.cached_input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.last_token_usage.input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.last_token_usage.output_tokens | per_event | 10, 20, 30, 40, 40, 40 |
| $.payload.info.last_token_usage.reasoning_output_tokens | per_event | 10, 20, 30, 40, 40, 40 |
| $.payload.info.last_token_usage.total_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.cache_write_input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.cached_input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.input_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.output_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.reasoning_output_tokens | monotonic | 100, 150, 200, 250, 250, 250 |
| $.payload.info.total_token_usage.total_tokens | monotonic | 100, 150, 200, 250, 250, 250 |

### Identity and timestamp paths

- Session: None observed; Event: None observed; Timestamp: $.timestamp

### Diagnostics

None observed

### Artifacts

- Manifest: codex/manifest.json
- Records: codex/records.jsonl

### Privacy validation

Validated before write.
