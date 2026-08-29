# Project Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a minimal runnable Tauri 2 application that proves the React-to-Rust summary boundary and establishes repeatable frontend and Rust checks.

**Architecture:** Vite hosts a small React/TypeScript frontend. Tauri owns the Windows process and exposes one `get_usage_summary` command from Rust; the frontend maps that typed response into a deliberately plain bootstrap screen. Provider discovery, persistence, background collection, tray behavior, and final overlay styling remain outside this slice.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, Vite, Vitest, Testing Library, plain CSS, npm

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Global Constraints

- Target Windows 11 with one Tauri application executable and no app-managed sidecar or background service.
- Keep all filesystem and future SQLite access in Rust.
- Expose typed summaries only; do not expose source paths, provider records, or conversational content to the overlay webview.
- Include no network client, telemetry, frontend state library, CSS framework, ORM, or background service.
- Stop this slice after the shell, typed placeholder summary boundary, and checks are working.

---

### Task 1: Runnable Tauri and React shell

**Files:**
- Create: `.gitignore`
- Create: `package.json`
- Create: `package-lock.json` via `npm install`
- Create: `index.html`
- Create: `tsconfig.json`
- Create: `tsconfig.app.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `src/vite-env.d.ts`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/App.test.tsx`
- Create: `src/styles.css`
- Create: `src/test/setup.ts`
- Create: `src/lib/usage-summary.ts`
- Create: `src/lib/usage-summary.test.ts`
- Create: `src-tauri/.gitignore`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: the approved `UsageSummary` contract from the design spec.
- Produces: Rust command `get_usage_summary() -> UsageSummary`; TypeScript function `getUsageSummary(): Promise<UsageSummary>`; React `App` bootstrap screen.

- [x] **Step 1: Add the frontend contract test**

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getUsageSummary } from "./usage-summary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("getUsageSummary", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("requests only the summary command", async () => {
    const summary = { state: "loading", todayTokens: 0, sourceHealth: [] };
    vi.mocked(invoke).mockResolvedValue(summary);

    await expect(getUsageSummary()).resolves.toEqual(summary);
    expect(invoke).toHaveBeenCalledWith("get_usage_summary");
  });
});
```

- [x] **Step 2: Run the focused test and confirm the contract module is missing**

Run: `npm test -- --run src/lib/usage-summary.test.ts`

Expected: FAIL because `src/lib/usage-summary.ts` does not exist.

- [x] **Step 3: Add the package, TypeScript, Vite, Vitest, HTML, and test setup files**

Configure npm scripts `dev`, `build`, `test`, `test:watch`, and `tauri`; use React TypeScript through Vite; ignore `src-tauri` in Vite's watcher; and load `@testing-library/jest-dom/vitest` from `src/test/setup.ts`.

- [x] **Step 4: Implement the TypeScript summary boundary**

```ts
import { invoke } from "@tauri-apps/api/core";

export type UsageState = "loading" | "active" | "idle" | "unavailable" | "stale";

export interface SourceHealth {
  provider: string;
  state: string;
}

export interface UsageSummary {
  state: UsageState;
  provider?: string;
  currentSessionTokens?: number;
  todayTokens: number;
  lastUpdatedAt?: string;
  sourceHealth: SourceHealth[];
}

export function getUsageSummary(): Promise<UsageSummary> {
  return invoke<UsageSummary>("get_usage_summary");
}
```

- [x] **Step 5: Run the focused frontend contract test**

Run: `npm test -- --run src/lib/usage-summary.test.ts`

Expected: PASS with one test.

- [x] **Step 6: Add the failing React shell test**

```tsx
import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { getUsageSummary } from "./lib/usage-summary";

vi.mock("./lib/usage-summary", () => ({ getUsageSummary: vi.fn() }));

beforeEach(() => {
  vi.mocked(getUsageSummary).mockResolvedValue({
    state: "loading",
    todayTokens: 0,
    sourceHealth: [],
  });
});

it("renders the bootstrap summary returned by Rust", async () => {
  render(<App />);
  expect(await screen.findByText("Loading")).toBeInTheDocument();
  expect(screen.getByText("Today: 0 tokens")).toBeInTheDocument();
});
```

- [x] **Step 7: Run the React shell test and confirm the component is missing**

Run: `npm test -- --run src/App.test.tsx`

Expected: FAIL because `src/App.tsx` does not exist.

- [x] **Step 8: Implement the bootstrap React shell**

Create `App.tsx` with local loading, success, and unavailable states. On mount, call `getUsageSummary`; render the state label, a current-session placeholder when unavailable, today's total, and a short bootstrap note. Create `main.tsx` to mount the app and `styles.css` with only legible neutral defaults.

- [x] **Step 9: Run the frontend tests and production build**

Run: `npm test -- --run`

Expected: PASS.

Run: `npm run build`

Expected: TypeScript and Vite complete successfully and emit `dist/`.

- [x] **Step 10: Add the failing Rust summary test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_summary_contains_no_provider_data() {
        let summary = get_usage_summary();

        assert_eq!(summary.state, UsageState::Loading);
        assert_eq!(summary.today_tokens, 0);
        assert!(summary.provider.is_none());
        assert!(summary.current_session_tokens.is_none());
        assert!(summary.last_updated_at.is_none());
        assert!(summary.source_health.is_empty());
    }
}
```

- [x] **Step 11: Run the Rust test and confirm the crate is absent**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the Rust crate has not been created.

- [x] **Step 12: Implement the minimal Tauri crate and command**

Define serializable `UsageState`, `SourceHealth`, and `UsageSummary` types in `src-tauri/src/lib.rs`. Implement `get_usage_summary` with the loading-state zero summary, register it in `tauri::generate_handler!`, and call the library's `run` function from `main.rs`. Configure a single 320 by 120 window and the minimum `core:default` capability; do not add tray, plugins, persistence, networking, or source access.

- [x] **Step 13: Run Rust checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS with one unit test.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [x] **Step 14: Run the complete bootstrap gate**

Run: `npm test -- --run`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 15: Commit the bootstrap**

Not executed: no commit was requested.

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts src src-tauri docs/superpowers/plans/2026-08-29-project-bootstrap.md
git commit -m "feat: bootstrap token tracing widget"
```
