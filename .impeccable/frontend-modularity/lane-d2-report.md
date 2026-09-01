# Lane D2 preparation report

Date: 2026-09-01  
Branch: `refactor/ui-ux`  
Gate: `READONLYPREPARATION`

## Gate and baseline

This preparation pass read the repository agreement, `CONTEXT.md`,
`PRODUCT.md`, `design/DESIGN_CLAUDE.md`, the approved architecture spec,
`docs/superpowers/specs/2026-09-01-frontend-modularity-design-update.md`,
the canonical frontend modularity plan (Task 6), and the D1/D2 lane brief.
It also inspected the maintained 16-screen runtime baseline report and the
current settings styles and component entry points.

The initial pass was read-only. After root opened a limited preparation gate,
only the three new, still-unimported settings CSS Modules listed below were
created. No existing TSX, entry, test, legacy stylesheet, package, Rust, or Git
file was modified by D2. D1's semantic token/theme files are now visible and B
has a reviewed settings commit, but D1's shared styling handoff and the root
integration gate for existing files remain pending.

Observed current state:

- `src/styles/settings.css` is 507 lines and global.
- `src/settings-main.tsx` imports `src/styles/index.css`.
- `src/styles/index.css` imports tokens, reset/base, widget, settings, and
  window-control styles into both Vite entries.
- The permitted D2 modules are currently unimported: `surface.module.css` is
  214 lines, `forms.module.css` is 141 lines, and `theme-picker.module.css` is
  88 lines.
- Settings TSX still uses legacy class strings so B's behavior tests can run
  before the CSS migration.
- The working tree contains parallel B changes; D2 must wait for the B review
  handoff and must not overwrite those edits.

## D2-owned files

Create:

- `src/styles/settings/surface.module.css`
- `src/styles/settings/forms.module.css`
- `src/styles/settings/theme-picker.module.css`
- `src/tests/styles/settings-modules.test.mjs`
- `src/tests/styles/window-bundle-isolation.test.mjs`

Modify only for CSS module imports/class composition, with no settings logic
or native event changes:

- `src/components/settings/AppearanceSection.tsx`
- `src/components/settings/ProviderVisibilitySection.tsx`
- `src/components/settings/SourceSettingsSection.tsx`
- `src/components/settings/SettingsSwitch.tsx`
- `src/components/settings/SettingsCloseButton.tsx`
- `src/components/settings/ThemeSelect.tsx`
- `src/components/settings/SettingsScreen.tsx`
- `src/settings-main.tsx`
- `src/tests/styles/panel-surfaces.test.mjs`

`SettingsActivityPanel.tsx` has no style hooks of its own; inspect it for
composition compatibility but do not change it unless the B handoff adds a
CSS class that needs a module map. `SettingsScreen.behavior.test.tsx`,
`SettingsScreen.structure.test.tsx`, and `AppearanceSection.test.tsx` are B's
behavior/structure tests. Their current legacy-class assertions must be
updated or explicitly handed off before D2 removes those class strings.

Retire only after import and bundle checks prove the files unused:

- `src/styles/settings.css` (D2 settings ownership)
- `src/styles/index.css` (shared legacy entry; root confirms both entries)

D1 owns the new global files, widget modules, shared modules, and the old
`base.css`, `tokens.css`, `widget.css`, and `window-controls.css` retirement.
D2 will not edit those files concurrently.

## Module ownership and selector map

The split follows responsibility rather than JSX file size. All new selectors
are local CSS Module selectors. State combinations use module class maps; no
`theme--*`, `settings-page--*`, `is-open`, or `is-on` global selector remains.

### `surface.module.css`

Move the settings shell and structural panel rules from the current file:

- lines 1–57: root geometry, padding, body scroll, stable scrollbar gutter,
  edge extension, WebKit track/button/thumb rules;
- lines 97–135: header, heading, title row, title, and subtitle;
- lines 136–159: close-button shell and icon;
- lines 172–213: form/section/section heading/card layout;
- lines 215–263: row and identity layout, including row label typography;
- the `max-width: 420px` root padding override from lines 494–498.

Suggested exported responsibilities are `root`, `body`, `header`, `heading`,
`titleRow`, `closeButton`, `closeIcon`, `form`, `section`,
`appearanceSection`, `sectionHeading`, `sectionTitle`, `card`, `row`,
`identity`, and `identityContent`. Keep element descendants local to this
module; do not select a child shared-module class from here.

### `forms.module.css`

Move the editable control and source/status rules:

- lines 161–170: status/error feedback (the form feedback seam);
- lines 277–315 and 490–492: switch, knob, on/active state, and press state;
- lines 317–381: source row, actions, health indicator, source path button,
  hover, and focus;
- lines 383–405: appearance row, divider, and labels;
- lines 500–506: narrow source-action gap and health visibility override.

Suggested exported responsibilities are `status`, `statusError`, `switch`,
`switchOn`, `switchKnob`, `sourceRow`, `sourceMain`, `sourceActions`,
`sourceHealth`, `sourceHealthDot`, `sourcePath`, `appearanceRow`, and
`appearanceLabel`. Same-module sibling dividers may use `+` selectors; no
selector should pierce the theme-picker or shared branding module.

### `theme-picker.module.css`

Move lines 407–488 without changing picker behavior:

- `picker`, `pickerOpen`, `button`, `chevron`, `menu`, and `option`;
- open-state button/chevron transforms;
- button focus ring and option hover/selected state.

`ThemeSelect.tsx` must focus the module button key after Escape, preserving the
current keyboard interaction. The menu remains bottom-aligned above the
button, with its existing z-index, padding, radius, and shadow.

### Shared seams

`WindowGrip` and `WindowResizeHandles` remain owned by D1's
`src/styles/shared/window-controls.module.css`. D2 only ensures
`settings-main.tsx` includes the shared controls style graph required by the
settings entry. Provider mark/name presentation remains in D1's shared
branding module. D2 must not target `.provider-name`, `.provider-dot`, or a
provider-specific class from a settings module.

The old settings display-name rule was context-specific:

- settings display-role name: `calc(var(--type-settings-meta) + 4px)`;
- widget display-role name: `calc(var(--type-widget-provider) + 4px)`.

D1 must expose this through its shared branding interface or an explicit local
class passed to the shared name element. A settings parent selector must not
style a child module's class.

## Token contract requested from D1

D1 owns names and declarations. The following semantic slots are required by
D2; the values are the current computed values that must remain unchanged.
Component modules consume the final D1 names directly and never use a
`var(--claude-...)` fallback or a raw Claude theme value.

| Semantic role | Required values/contract | D2 consumers |
| --- | --- | --- |
| canvas/card | light `#faf9f5` / `#efe9de`; dark `#181715` / `#252320` | root, card, picker menu |
| ink/body/muted | light `#141413` / `#3d3d3a` / `#6c6a64`; dark `#faf9f5` / `#d0cbc2` / `#a09d96` | shell, headings, labels, status |
| muted-soft/line/line-soft | light `#8e8b82` / `#e6dfd8` / `#ebe6df`; dark `#8f8b83` / `#3a3630` / `#37332d` | metadata, dividers, scrollbar |
| accent/active/soft/positive | coral `#cc785c`, active `#a9583e`, soft `rgba(204,120,92,.28)`, teal `#5db8a6` | links, focus, health, selected option |
| switch track/knob | off track light `#d7cfc6`, dark `#4b4740`; knob light `#fff`, dark `#faf9f5` | settings switch |
| focus | existing global focus blue `#0071e3`; settings accent outline and 3px soft ring | reset, path, picker |
| elevation | settings light `0 18px 42px -16px rgba(20,20,19,.26), 0 36px 82px -26px rgba(20,20,19,.24)`; dark `0 18px 42px -16px rgba(0,0,0,.54), 0 36px 82px -26px rgba(0,0,0,.48)` | settings root |
| menu elevation | `0 10px 24px -14px rgba(0,0,0,.65)` | theme picker menu |
| type | system UI/display/mono stacks; settings title 32px, section 20px, label 14px, meta 12px | all settings modules |
| geometry/rhythm | inline padding 32px (narrow 22px), scrollbar gutter/width 8px, thumb border 2px, row 60px, sections 28px/24px, card radius 12px, shell radius 16px | surface/forms |
| controls timing | `140ms ease` transitions; reduced-motion reset remains global | switch, picker |

Theme values must be selected by D1's root-scoped `[data-theme="claude"]`
and `[data-color-mode="light"|"dark"]` declarations. The settings root
keeps `data-theme="claude"` and `data-color-mode` and does not restore
legacy theme classes for styling.

## Entry and isolation contract

`src/settings-main.tsx` must import the global foundation (`reset.css`,
`tokens.css`, `themes.css`), the three settings modules, and the D1 shared
window-controls/branding modules required by this entry. It must not import
widget modules. `src/main.tsx` must not import settings modules.

After explicit imports are in place, the generated settings bundle may contain
only globals, settings modules, and shared modules. It must contain no
`widget/` module or old `settings.css`/`index.css` graph. The widget bundle
must contain no settings module.

## Verification to run after the gate opens

Focused D2 command:

```powershell
npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/settings-modules.test.mjs src/tests/styles/window-bundle-isolation.test.mjs src/tests/components/settings
```

Build and source checks:

```powershell
npm run build
rg -n "var\(--claude-|:global|provider-(?:name|dot)--(?:claude|codex)|\.provider-(?:claude|codex)" src/styles/settings src/styles/widget src/styles/shared
rg -n "styles/(index|settings|widget|base|tokens|window-controls)|\.module\.css" src/main.tsx src/settings-main.tsx src/components/settings src/components/shared src/components/widget
Get-ChildItem src -Recurse -File -Include *.ts,*.tsx,*.css,*.mjs | ForEach-Object { $count = (Get-Content $_.FullName).Count; if ($count -gt 250) { "OVER 250: $($_.FullName) $count" } }
git diff --check
```

`settings-modules.test.mjs` must assert the three modules contain only local
selectors, no provider-named selectors, no `var(--claude-` fallback, no
`:global`, and no parent piercing. `window-bundle-isolation.test.mjs` must
assert entry import disjointness. `panel-surfaces.test.mjs` must preserve the
runtime scrollbar edge, stable gutter, hidden WebKit arrow, borderless root,
diffuse negative-spread elevation, theme picker, provider branding, and
shadow-free widget assertions against the new module sources.

The 16 synthetic browser baseline screens remain comparison evidence only:
settings light/dark at 520x600 and 380x600, plus all widget light/dark
0/1/2-provider normal/narrow sizes. They do not prove Windows native drag,
resize, DPI, picker, persistence, or close-flush behavior. Root owns the
packaged Windows smoke check after D2.

## Limits and handoff

- No Rust, Tauri capabilities, bridge contracts, hooks, settings persistence,
  provider registry, session payload, dependency, or theme behavior changes.
- No new CSS framework, font, global store, network client, or sidecar.
- No static preview recreation and no Linux/macOS claim.
- No staging, commit, push, worktree, or Git cleanup by D2.
- No existing source mutation before D1's reviewed shared styling handoff and
  root's existing-file integration gate; the three new CSS Modules above are
  the only permitted preparation edits.
- All new CSS and test source files target 80–200 lines and must stay at or
  below the repository's 250-line limit.

The token request and cross-module branding seam were sent to root. Root must
relay the token requirements to D1 if the D1 worker does not see the report
directly, then explicitly release the D2 implementation gate.
