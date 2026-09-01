# Session file → frontend: phân tích end-to-end

Bộ tài liệu này mô tả **đường đi thực tế trong code hiện tại** từ session file của Claude Code/Codex đến widget React. Snapshot được ghim tại commit `4f6dece2599b09449e5540d286c29bd08f199dc2`.

Phạm vi: Windows 11, local-only, metadata-only. Filesystem, discovery, parser, normalization, SQLite và aggregate đều ở Rust. React chỉ nhận `UsageSummary` đã chuẩn hóa; raw prompt, response, reasoning, tool payload, credential, repository content, working directory và raw path không đi qua wire.

## Bộ artifact

| File | Loại | Câu hỏi chính |
|---|---|---|
| [`01-system-overview.html`](01-system-overview.html) | Architecture | Module, boundary và đường dữ liệu toàn tuyến |
| [`02-startup-to-frontend.html`](02-startup-to-frontend.html) | Sequence | Startup one-shot → commit → query → render đầu |
| [`03-live-update-and-recovery.html`](03-live-update-and-recovery.html) | Sequence | File change → debounce/reconcile → tail-read → publish/retry |
| [`04-read-normalize-aggregate.html`](04-read-normalize-aggregate.html) | Dataflow | Byte/JSON → observation → event → aggregate → payload |
| [`05-sqlite-storage.html`](05-sqlite-storage.html) | Architecture | Bảng, transaction, checkpoint và omission boundary |
| [`06-scheduler-and-summary.html`](06-scheduler-and-summary.html) | Lifecycle | Startup, watching, active, idle, stale, retry, unavailable |

Các file `*.json` là candidate source. Các file `*.visual-check.json`, `*.visual-check.html` và screenshot là browser evidence sau deliver, không phải runtime asset của ứng dụng.

## Call graph tổng quát

```text
Tauri setup
  → initialize AppState + IndexStore + settings
  → collect_once(WindowsClock::current())
  → discover safe provider roots
  → read selected files from checkpoints
  → adapter → TokenObservation
  → validate + delta conversion + event identity
  → one SQLite apply_batch transaction
  → query events ≤ now
  → Rust session/provider/day summary
  → emit usage-summary-changed

Changed(provider) / 30-second reconciliation / config change
  → debounce or retry deadline
  → re-discover
  → same read → normalize → SQLite → summary → publish path

React
  → install event listener first
  → invoke get_usage_summary()
  → strict parse
  → settings/preview transform
  → provider rows + Total + native height sync
```

Frontend không đọc file và không tự tính token. Mọi con số nghiệp vụ được tính xong trong Rust trước boundary Tauri.

## 1. Startup path

### 1.1 Clock và runtime

`src-tauri/src/lib.rs` thực hiện một collection sau setup. `WindowsClock::current()` tách hai khái niệm:

- UTC now từ `SystemTime`, dùng epoch seconds/ISO `YYYY-MM-DDTHH:mm:ssZ` cho cutoff và activity.
- Windows-local day từ `GetLocalTime`, dùng để tính `Today`.

`runtime.rs` tạo `AppState`, settings và `IndexStore` ở app-local-data directory, đăng ký Claude/Codex adapters, resolve roots và gọi coordinator. Hai provider được xử lý độc lập; một provider fail không chặn provider kia.

### 1.2 Discovery không đọc nội dung

`session_files.rs`:

1. Resolve default roots dưới `USERPROFILE`, gồm `.claude/projects` và `.codex/sessions`, hoặc dùng explicit root đã validate.
2. Từ chối relative/parent traversal, arbitrary UNC/device path và reparse escape; WSL chỉ qua dạng được allow.
3. Walk bằng `symlink_metadata`; chỉ regular `.json`/`.jsonl` được chọn.
4. Giữ private full path, sanitized relative pattern, size và mtime.
5. Sort newest mtime trước, relative pattern làm tie-break.
6. Greedy-select theo full file size cho tới budget mặc định 50 MiB/provider; default không đặt file-count cap hữu hạn.
7. Tạo `opaque_identity = SHA-256(provider + NUL + filesystem_path)`.

Mtime chỉ dùng cho ordering/checkpoint compatibility. Nó không phải thời điểm token được dùng và không đi ra frontend.

### 1.3 Checkpoint và bounded reader

Checkpoint theo file identity giữ `next_offset`, pending offset, size/mtime snapshot, monotonic segment và Codex cumulative baselines. File lớn lên tương thích thì seek tới offset cũ; truncate/rewrite không tương thích thì bắt đầu state/segment mới.

`provider_adapter.rs` dùng `BufReader`, `seek(start_offset)` và giới hạn `MAX_RECORD_BYTES = 1 MiB`. `source_position` là byte offset đầu line.

- Blank line: offset tiến, không có observation.
- JSON hợp lệ: parse trong memory, đưa qua adapter, offset tiến.
- Shape không liên quan: parser trả `None`, bỏ qua nhưng offset vẫn tiến.
- Malformed complete line: file attempt fail, không sinh UsageEvent hợp lệ.
- Final line chưa hoàn chỉnh: giữ pending offset tại record start, lần sau đọc lại.

Không raw line/raw JSON nào được giữ trong normalized event, SQLite, diagnostics hay frontend payload.

### 1.4 Claude và Codex parser

Claude chỉ nhận `message.type == "message"` có `message.usage`, input/output, optional cache-read, timestamp, session key từ `sessionId`/`session_id`, event key từ `message.id`/`uuid`. Counter kind là `incremental`; total được kiểm tra theo input + output.

Codex chỉ nhận `payload.type == "token_count"` có `info.total_token_usage`, input/output, optional total/cached và timestamp. Counter kind là `cumulative`; record là snapshot lũy kế. Không có provider session/event key tương đương Claude nên file identity thường là fallback session boundary.

Cả hai trả provider-neutral `TokenObservation` rồi mới qua collection core.

## 2. Normalize, delta, dedupe

### 2.1 Validation và thứ tự

Collection kiểm tra counter không âm, total nhất quán, timestamp có thể dùng và identity/source position đủ. Observations được sort theo `observed_at`, sau đó `source_position`; cùng input cho ra thứ tự deterministic.

Effective session key là provider session key nếu có, nếu không là opaque file identity. Event identity là hash của provider/session-event key khi provider cung cấp; nếu không thì hash của file identity + source position + counter kind.

### 2.2 Incremental Claude

Với Claude, normalized event lấy trực tiếp input, cache-read, output; `total = input + output`; counter kind/segment được ghi cùng event. Không trừ baseline. Observation zero-token có thể chỉ làm checkpoint tiến mà không tạo fact token zero.

### 2.3 Cumulative Codex

Với snapshot hiện tại `C` và baseline `P`:

```text
delta.input  = C.input  - P.input
delta.cache  = C.cache  - P.cache
delta.output = C.output - P.output
delta.total  = C.total  - P.total
```

Nếu **bất kỳ** counter nào giảm, coi là reset/rotation:

```text
segment = segment + 1
delta = current snapshot C
baseline = C
```

Như vậy không sinh số âm. Baseline mới được lưu cùng checkpoint trong batch.

### 2.4 Dedupe qua restart

`seen_event_ids` trong memory bị giới hạn khoảng 4096 id cho một lần collection. Checkpoint SQLite hiện không persist seen-set. Restart safety thực tế đến từ `usage_events.event_id` là primary key, `INSERT OR IGNORE`, cùng offset/baseline checkpoint.

`accepted_event_count` là batch length trước khi SQLite ignore duplicate, nên không nhất thiết bằng số row mới insert.

## 3. SQLite và summary

### 3.1 Một transaction ghi gì?

`IndexStore::apply_batch` ghi normalized `usage_events`, upsert `sessions`, provider `sources`, sanitized `diagnostics` và `file_checkpoints` trong một transaction. Write error làm rollback; summary chỉ được tính/publish sau commit.

| Bảng | Vai trò |
|---|---|
| `usage_events` | event id, provider, file/session key, source position, timestamp, kind, segment, token fields |
| `file_checkpoints` | offset, size/mtime, pending, segment, cumulative baselines |
| `sessions` | provider/session metadata, min start/max activity |
| `sources` | source health/status đã sanitize |
| `diagnostics` | category/count/error state đã sanitize |
| `settings` | persisted app settings |

Không bảng nào là raw provider-record store.

### 3.2 Query hiện tại

Collector gọi `query_events_for_summary("", now)`. `day_start` đang empty nên SQL đọc mọi event có `observed_at <= now`; Rust mới parse/filter local day và group. Đây là history read + recompute, không phải query “row mới nhất”.

Rust sau đó:

1. loại event timestamp không parse được hoặc lớn hơn `now`;
2. đổi timestamp sang UTC/epoch và local day;
3. group `(provider, effective_session_key)`;
4. chọn latest bằng timestamp/source position/event id;
5. tính session total, Today, current session, active/idle và provider/top-level health.

`sessions` được duy trì nhưng summary hiện đọc `usage_events` để aggregate.

## 4. Time semantics chi tiết

| Giá trị | Nguồn | Dùng cho | Không dùng cho |
|---|---|---|---|
| file mtime | filesystem metadata | newest-first, checkpoint compatibility | active age, Today |
| `observed_at` | provider record timestamp | ordering, cutoff, latest, age, local-day | scheduler deadline |
| collection `now` | `WindowsClock`/injected clock | `event <= now`, activity | file sort |
| Windows local day | `GetLocalTime`/timezone API | Today/current-session day | sửa timestamp gốc |
| scheduler `Instant` | monotonic clock | debounce 200 ms, reconcile 30 s, retry | SQLite/frontend |
| frontend `Date.now()` | browser clock | relative “ago” label | Rust totals |

Các công thức chính:

```text
eligible(event) = event_timestamp <= collection_now
age             = collection_now - latest_event_timestamp
active          = latest valid event within 120 seconds
event_day       = date(UTC(event_timestamp) + current Windows offset)
today(event)    = event_day == current Windows-local day
```

Current implementation dùng current timezone offset cho historical timestamp, nên có caveat ở mốc DST nếu historical offset khác offset hiện tại. Frontend không có interval timer cho relative label; label đổi khi có rerender.

## 5. Aggregation và lifecycle

Một session group có tổng token saturating, latest event và day total. Nếu latest valid event còn trong 120 giây thì active; quá cửa sổ thì idle nhưng totals/last update lịch sử vẫn giữ.

- Active: có activity trong cửa sổ 120 giây.
- Idle: không active nhưng còn history/summary dùng được.
- Stale: collection/query fail; previous summary được copy, chỉ đổi state thành stale.
- Unavailable: không usable source và không có provider history đủ để hiển thị.

Active-provider selector chọn latest valid event `<= now`, nhưng không phải lúc nào cũng áp 120-second filter cho field provider/lastUpdatedAt. Vì vậy top-level có thể Idle trong khi provider/lastUpdatedAt vẫn là provider gần nhất.

Scheduler path:

| Nhánh | Kết quả |
|---|---|
| Startup | one-shot collect |
| `Changed(provider)` | debounce 200 ms, re-discover |
| reconciliation | mỗi 30 s bằng `Instant` |
| config change | đổi watcher roots rồi queue provider |
| read/parse/storage/query fail | diagnostics + retry, không publish batch lỗi |
| retry | 1 → 2 → 4 → 8 → 16 → 30 s, cap 30 s |
| publisher fail sau commit | durable state giữ nguyên; không retry riêng publisher |

Watcher phát generic provider signal, không forward filename/path. Re-discovery luôn chạy lại safety rules.

## 6. Tauri wire và frontend

Rust serialize camelCase `UsageSummary` gồm `state`, optional `provider`, optional `currentSessionTokens`, `todayTokens`, optional `lastUpdatedAt`, `sourceHealth` và fixed provider summaries Claude/Codex.

Có hai đường nhận:

1. `get_usage_summary`: trả last committed summary; không tự collection.
2. `usage-summary-changed`: emit sau commit + query thành công.

`useUsageSummary` cài listener trước rồi invoke command, nên startup event race vẫn được recover bởi command. Hook bắt đầu loading; command/event fail hoặc invalid payload rơi về unavailable.

TypeScript parser:

- reject unknown keys;
- state phải thuộc union;
- token phải là safe integer không âm;
- date phải hợp lệ nếu có;
- provider id phải được biết;
- providers phải đủ đúng canonical count, không duplicate;
- source health chỉ giữ provider + state string đã sanitize.

`createWidgetViewModel` áp `visibleProviders`, canonical order và preview-disabled state. Row hiển thị provider identity, status, `Session`, `Today`, relative update. `Total` dùng Rust `summary.todayTokens`; trong settings preview, provider disabled bị loại khỏi preview Total.

Widget không hiển thị session id, absolute path, raw record hay SQLite row. Token format dùng `toLocaleString("en-US")`; undefined là `Unavailable`. Relative bucket: `No updates yet`, `just now`, `N min ago`, `N hr ago`, `N d ago`.

## 7. Source map

| Trách nhiệm | Source |
|---|---|
| startup | [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs#L29) |
| runtime | [`src-tauri/src/app/runtime.rs`](../src-tauri/src/app/runtime.rs#L91) |
| live scheduler/retry | [`src-tauri/src/app/live_collection.rs`](../src-tauri/src/app/live_collection.rs#L25) |
| discovery | [`src-tauri/src/sources/session_files.rs`](../src-tauri/src/sources/session_files.rs#L148) |
| watcher | [`src-tauri/src/sources/file_watcher.rs`](../src-tauri/src/sources/file_watcher.rs#L29) |
| reader | [`src-tauri/src/providers/provider_adapter.rs`](../src-tauri/src/providers/provider_adapter.rs#L70) |
| Claude parser | [`src-tauri/src/providers/claude/record_parser.rs`](../src-tauri/src/providers/claude/record_parser.rs#L9) |
| Codex parser | [`src-tauri/src/providers/codex/record_parser.rs`](../src-tauri/src/providers/codex/record_parser.rs#L9) |
| collection/delta | [`src-tauri/src/collection/mod.rs`](../src-tauri/src/collection/mod.rs#L244), [`src-tauri/src/usage/cumulative_delta.rs`](../src-tauri/src/usage/cumulative_delta.rs#L41) |
| aggregation | [`src-tauri/src/usage/session_summary.rs`](../src-tauri/src/usage/session_summary.rs#L29) |
| SQLite | [`src-tauri/src/database/connection.rs`](../src-tauri/src/database/connection.rs#L99), [`src-tauri/src/database/schema.rs`](../src-tauri/src/database/schema.rs#L10) |
| wire | [`src-tauri/src/types/usage_summary.rs`](../src-tauri/src/types/usage_summary.rs#L22), [`src-tauri/src/commands/usage_summary.rs`](../src-tauri/src/commands/usage_summary.rs#L9) |
| frontend parser/hook | [`src/lib/contracts/usage-summary.ts`](../src/lib/contracts/usage-summary.ts#L24), [`src/hooks/useUsageSummary.ts`](../src/hooks/useUsageSummary.ts#L27) |
| view/render | [`src/lib/widget-view-model.ts`](../src/lib/widget-view-model.ts#L51), [`src/components/widget/TokenTracingWidget.tsx`](../src/components/widget/TokenTracingWidget.tsx#L13) |

## Verification

- `npm test -- --run`: 28 test files, 93 tests passed.
- `npm run build`: passed.
- Rust fmt/check: passed.
- Rust target analysis tests: 112 passed; target riêng dùng vì app binary đang chạy giữ lock target mặc định.
- Archify: 6/6 validate showcase sạch; mỗi deliver 9/9 artifact checks, 0 error/0 warning.
- Edge browser visual-check: 6/6 pass cho containment, readability, viewer chrome và captures ở các viewport kiểm tra.
- Đã spot-check screenshot light 1440×900 của cả sáu sơ đồ.

Sơ đồ dùng editorial preset mặc định của Archify/diagram-design. HTML standalone mở trực tiếp được; guided views giúp focus từng path mà không đổi canonical geometry.
