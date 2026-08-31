# Panel edge and shadow correction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the settings scrollbar to the panel edge and remove the crisp
CSS perimeter artifact while preserving visible depth on the widget and
settings surfaces.

**Architecture:** Keep the settings root as the fixed flex shell. Extend only
its scroll body into the shell's right padding and restore the content inset
with body padding. Keep native Tauri shadow disabled and share the same diffuse
negative-spread CSS elevation contract between runtime styles and the static
Claude review surface.

**Tech Stack:** React 19, TypeScript, Vite, Vitest, plain CSS, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-08-31-panel-edge-shadow-correction.md`

## Global Constraints

- Keep version one Windows 11-only, local-only, and metadata-only.
- Keep filesystem, collection, source discovery, and SQLite access in Rust.
- Preserve the typed native drag/resize bridge and `shadow: false` window
  configuration.
- Do not add dependencies, fonts, CSS frameworks, or frontend state libraries.
- Keep generated `.impeccable/` output and build artifacts out of the change.

---

### Task 1: Lock the panel CSS contracts with failing tests

**Files:**
- Create: `src/tests/styles/panel-surfaces.test.mjs`
- Test: `src/styles/settings.css`, `src/styles/widget.css`,
  `src/design-preview.css`

**Interfaces:**
- Consumes: the existing stylesheet selectors for runtime and static panels.
- Produces: regression checks for scrollbar edge alignment, hidden WebKit
  scrollbar buttons, borderless panel surfaces, and diffuse shadows.

- [ ] **Step 1: Write the failing test**

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");

const block = (css: string, selector: string) => {
  const escaped = selector.replace(/[.*+?^${}()|[\\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`));
  if (!match) throw new Error(`Missing CSS selector: ${selector}`);
  return match[1];
};

const settingsCss = read("../../styles/settings.css");
const widgetCss = read("../../styles/widget.css");
const previewCss = read("../../design-preview.css");

describe("panel surface CSS", () => {
  it("keeps the settings scrollbar at the shell edge", () => {
    const body = block(settingsCss, ".settings-page__body");
    expect(body).toMatch(/margin-right:\s*calc\\(-1\\s*\\*\\s*var\\(--settings-inline-padding\\)\\)/);
    expect(body).toMatch(/padding-right:\s*var\\(--settings-inline-padding\\)/);
    expect(body).toMatch(/scrollbar-gutter:\s*stable/);
    expect(settingsCss).toMatch(/\.settings-page__body::-webkit-scrollbar-button\\s*\\{[^}]*display:\s*none/);

    const previewBody = block(previewCss, ".claude-settings-body");
    expect(previewBody).toMatch(/margin-right:\s*calc\\(-1\\s*\\*\\s*var\\(--settings-inline-padding\\)\\)/);
    expect(previewBody).toMatch(/padding-right:\s*var\\(--settings-inline-padding\\)/);
  });

  it("uses borderless diffuse elevation without a perimeter line", () => {
    for (const css of [settingsCss, widgetCss]) {
      expect(css).not.toContain("0 1px 2px");
      expect(css).toMatch(/box-shadow:[^;{}]*-\\d+px/);
      expect(css).toMatch(/border:\s*0/);
      expect(css).toMatch(/outline:\s*none/);
    }

    expect(previewCss).not.toContain("0 1px 2px");
  });
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `npm test -- --run src/tests/styles/panel-surfaces.test.mjs`

Expected: FAIL because the current settings body has no edge extension and
the current panel shadows still contain the crisp `0 1px 2px` layer.

### Task 2: Implement the minimal edge and elevation correction

**Files:**
- Modify: `src/styles/settings.css`
- Modify: `src/styles/widget.css`
- Modify: `src/design-preview.css`

**Interfaces:**
- Consumes: the existing `--settings-*`, `--claude-*`, and panel selectors.
- Produces: identical borderless, diffuse elevation in runtime and static
  preview, plus a stable edge-aligned settings scrollbar.

- [ ] **Step 1: Extend only the settings scroll body into the shell gutter**

Add `--settings-inline-padding: 32px` to `.settings-page`, use it for the
horizontal shell padding, then add the following to both settings scroll body
selectors:

```css
margin-right: calc(-1 * var(--settings-inline-padding));
padding-right: var(--settings-inline-padding);
```

Add the 420px override `--settings-inline-padding: 22px` and hide
`::-webkit-scrollbar-button` with `display: none; width: 0; height: 0`.

- [ ] **Step 2: Replace the crisp shadow layer in all panel surfaces**

Use these exact dark/light contracts in runtime and static preview panel
selectors:

```css
/* dark */
box-shadow: 0 18px 42px -16px rgba(0, 0, 0, 0.54),
  0 36px 82px -26px rgba(0, 0, 0, 0.48);

/* light */
box-shadow: 0 18px 42px -16px rgba(20, 20, 19, 0.26),
  0 36px 82px -26px rgba(20, 20, 19, 0.24);
```

Keep `border: 0`, `outline: none`/`outline: 0`, and `shadow: false` unchanged.

- [ ] **Step 3: Run the focused test to verify it passes**

Run: `npm test -- --run src/tests/styles/panel-surfaces.test.mjs`

Expected: PASS.

### Task 3: Verify the complete slice

**Files:**
- Modify: `CONTEXT.md` only if the maintained UI contract needs the new
  scrollbar/shadow correction recorded.

- [ ] **Step 1: Run frontend checks**

Run: `npm test -- --run` and `npm run build`

Expected: PASS with all frontend tests green and a successful Vite build.

- [ ] **Step 2: Run native checks and package the debug artifact**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
`cargo check --manifest-path src-tauri/Cargo.toml`,
`cargo test --manifest-path src-tauri/Cargo.toml`, and
`npm run tauri build -- --debug`.

Expected: PASS; native shadow remains disabled in the generated debug app.

- [ ] **Step 3: Run visual/source hygiene checks**

Run `git diff --check` and the Impeccable detector against the changed
stylesheets and UI test. Confirm no `.impeccable/` or generated build output
is staged.

Expected: no diff whitespace errors, no detector findings, and only intended
source/spec/plan changes are present.
