// THROWAWAY DESIGN PROTOTYPE
// Question: what should the multi-provider overlay and Settings screen look like?
// Three variants live on this route and switch via ?variant=A|B|C.

const variants = [
  { key: "A", name: "macOS Settings", description: "Grouped rows + focus canvas" },
  { key: "B", name: "Focus first", description: "Total-led single canvas" },
  { key: "C", name: "Inspector", description: "Two-column control surface" },
  { key: "D", name: "Claude editorial", description: "Warm canvas + dark surface" },
];

const providers = {
  claude: {
    name: "Claude Code",
    shortName: "Claude",
    accent: "#d97757",
    session: "42,184",
    today: "147,271,872",
    state: "Idle",
    updated: "3 min ago",
    root: ".claude/projects",
  },
  codex: {
    name: "Codex",
    shortName: "Codex",
    accent: "#7e9bff",
    session: "183,256",
    today: "26,544,812",
    state: "Active",
    updated: "just now",
    root: ".codex/sessions",
  },
};

const root = document.querySelector("#design-preview-root");
const params = new URLSearchParams(window.location.search);
let currentKey = variants.some(({ key }) => key === params.get("variant"))
  ? params.get("variant")
  : "A";
const visibility = { claude: true, codex: true };
let darkMode = true;

function icon(name, className = "ui-icon") {
  const paths = {
    display: '<rect x="3" y="3" width="18" height="18" rx="4"></rect><path d="M8 12h8M12 8v8"></path>',
    sources: '<path d="M4 7h16M4 12h16M4 17h16"></path><circle cx="8" cy="7" r="1.5"></circle><circle cx="16" cy="12" r="1.5"></circle><circle cx="10" cy="17" r="1.5"></circle>',
    overlay: '<rect x="4" y="5" width="16" height="14" rx="3"></rect><path d="M8 9h8M8 13h5"></path>',
    search: '<circle cx="10.8" cy="10.8" r="5.8"></circle><path d="m15.2 15.2 4.3 4.3"></path>',
    back: '<path d="m14.5 5-7 7 7 7"></path>',
    forward: '<path d="m9.5 5 7 7-7 7"></path>',
    close: '<path d="m7 7 10 10M17 7 7 17"></path>',
    check: '<path d="m5 12 4 4L19 6"></path>',
    previous: '<path d="m15 5-7 7 7 7"></path>',
    next: '<path d="m9 5 7 7-7 7"></path>',
  };
  return `<svg class="${className}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`;
}

function providerDot(provider) {
  return `<span class="provider-dot" style="--provider-accent: ${provider.accent}" aria-hidden="true"></span>`;
}

function windowGrip() {
  return `<span class="window-grip" aria-hidden="true">${Array.from(
    { length: 6 },
    () => '<span class="window-grip__dot"></span>',
  ).join("")}</span>`;
}

function switchControl(providerId, label, checked = true) {
  const provider = providers[providerId];
  const ariaLabel = provider ? `${label} ${provider.name}` : label;
  return `
    <button
      class="switch ${checked ? "is-on" : ""}"
      type="button"
      role="switch"
      aria-checked="${checked}"
      aria-label="${ariaLabel}"
      data-switch="${providerId}"
    ><span class="switch__knob"></span></button>`;
}

function providerControl(providerId, { compact = false, includeRoot = true } = {}) {
  const provider = providers[providerId];
  const supportingText = includeRoot
    ? `<span>${provider.root}</span>`
    : "";
  return `
    <div class="provider-control ${compact ? "provider-control--compact" : ""}">
      <div class="provider-control__identity">
        ${providerDot(provider)}
        <div>
          <strong>${provider.name}</strong>
          ${supportingText}
        </div>
      </div>
      <div class="provider-control__actions">
        ${switchControl(providerId, "Show in widget", visibility[providerId])}
      </div>
    </div>`;
}

function sourceControl(providerId) {
  const provider = providers[providerId];
  return `
    <div class="source-control">
      <div class="source-control__identity">
        ${providerDot(provider)}
        <div>
          <strong>${provider.name}</strong>
          <span>${provider.root}</span>
        </div>
      </div>
      <div class="source-control__actions">
        <span class="source-health"><span class="health-dot"></span>Detected</span>
        ${switchControl(providerId, "Collect source for", true)}
      </div>
    </div>`;
}

function macSourceControl(providerId) {
  const provider = providers[providerId];
  return `
    <div class="mac-source-row">
      <div class="mac-source-row__identity">
        ${providerDot(provider)}
        <div><strong>${provider.name}</strong><span>${provider.root}</span></div>
      </div>
      <div class="mac-source-row__actions">
        <span class="source-health"><span class="health-dot"></span>Ready</span>
        ${switchControl(providerId, "Collect source for", true)}
        <button class="mac-change-button" type="button">Change…</button>
      </div>
    </div>`;
}

function appearanceControl() {
  return `
    <section class="settings-section settings-section--appearance">
      <div class="section-heading"><div><h2>Appearance</h2></div></div>
      <div class="appearance-list">
        <div class="appearance-row">
          <strong>Dark mode</strong>
          ${switchControl("dark-mode", "Dark mode", darkMode)}
        </div>
      </div>
    </section>`;
}

function claudeAppearanceControl() {
  return `
    <section class="claude-settings-section claude-settings-section--appearance">
      <div class="claude-section-heading"><h2>Appearance</h2></div>
      <div class="claude-settings-card claude-appearance-card">
        <div class="claude-appearance-row">
          <strong>Dark mode</strong>
          ${switchControl("dark-mode", "Dark mode", darkMode)}
        </div>
      </div>
    </section>`;
}

function claudeProviderControl(providerId) {
  const provider = providers[providerId];
  return `
    <div class="claude-provider-row">
      <div class="claude-provider-row__identity">
        ${providerDot(provider)}
        <div><strong>${provider.name}</strong><span>${provider.state} · ${provider.updated}</span></div>
      </div>
      ${switchControl(providerId, "Show in overlay", visibility[providerId])}
    </div>`;
}

function claudeSourceControl(providerId) {
  const provider = providers[providerId];
  return `
    <div class="claude-source-row">
      <div class="claude-source-row__identity">
        ${providerDot(provider)}
        <div><strong>${provider.name}</strong><span>${provider.root}</span></div>
      </div>
      <div class="claude-source-row__actions">
        <span class="claude-source-health"><span class="health-dot"></span>Ready</span>
        ${switchControl(providerId, "Collect source for", true)}
        <button class="claude-change-button" type="button">Change…</button>
      </div>
    </div>`;
}

function overlayRow(providerId) {
  const provider = providers[providerId];
  return `
    <div class="overlay-provider ${visibility[providerId] ? "" : "is-hidden"}" data-overlay-provider="${providerId}">
      <div class="overlay-provider__heading">
        <div class="overlay-provider__name">${providerDot(provider)}<strong>${provider.name}</strong></div>
        <span class="overlay-status overlay-status--${provider.state.toLowerCase()}"><span></span>${provider.state}</span>
      </div>
      <div class="overlay-provider__metrics">
        <div><span>Session</span><strong>${provider.session}</strong></div>
        <div><span>Today</span><strong>${provider.today}</strong></div>
        <span class="overlay-provider__updated">${provider.updated}</span>
      </div>
    </div>`;
}

function overlayPreview() {
  const claudeClass = currentKey === "D" ? " overlay-preview--claude" : "";
  const themeClass = darkMode ? " overlay-preview--dark" : " overlay-preview--light";
  return `
    <section class="overlay-preview${claudeClass}${themeClass}" aria-label="Overlay preview">
      <div class="overlay-preview__chrome">
        <span>Overlay preview</span>
        <span class="preview-chip">Sample data</span>
      </div>
      <div class="overlay-window">
        <header class="overlay-window__header">
          <div class="overlay-window__title">${windowGrip()}<strong>Token Tracing</strong></div>
          <div class="overlay-status overlay-status--active"><span></span>Live</div>
        </header>
        <div class="overlay-window__rule"></div>
        <div class="overlay-provider-list">
          ${overlayRow("claude")}
          ${overlayRow("codex")}
        </div>
        <div class="overlay-window__total"><span>Total</span><strong>173,816,684</strong></div>
      </div>
      <p class="overlay-preview__note">The widget keeps both providers visible even when one is idle.</p>
    </section>`;
}

function variantA() {
  return `
    <section class="settings-window settings-window--a">
      <main class="settings-content settings-content--mac settings-content--mac-no-sidebar">
        <div class="settings-topbar settings-topbar--mac"><div class="mac-navigation"><span>Settings</span></div><button class="close-button" type="button" aria-label="Close settings">${icon("close")}</button></div>
        <section class="settings-section">
          <div class="section-heading"><div><h2>Visible providers</h2></div><span class="section-count">2 selected</span></div>
          <div class="provider-list">${providerControl("claude", { includeRoot: false })}${providerControl("codex", { includeRoot: false })}</div>
        </section>
        <section class="settings-section settings-section--sources">
          <div class="section-heading"><div><h2>Sources</h2></div></div>
          <div class="source-list">${macSourceControl("claude")}${macSourceControl("codex")}</div>
        </section>
        ${appearanceControl()}
        <div class="settings-actions"><button class="primary-button" type="button">Save changes</button></div>
      </main>
    </section>`;
}

function variantD() {
  return `
    <section class="settings-window settings-window--d ${darkMode ? "settings-window--d-dark" : "settings-window--d-light"}">
      <main class="claude-settings-content">
        <header class="claude-settings-panel-header">
          <div>
            <div class="claude-settings-panel-title-row">${windowGrip()}<h1>Settings</h1></div>
            <p>Choose what stays visible.</p>
          </div>
          <button class="claude-settings-close-button" type="button" aria-label="Close settings">${icon("close")}</button>
        </header>
        <section class="claude-settings-section">
          <div class="claude-section-heading"><h2>Visible providers</h2></div>
          <div class="claude-settings-card">
            ${claudeProviderControl("claude")}
            ${claudeProviderControl("codex")}
          </div>
        </section>
        <section class="claude-settings-section">
          <div class="claude-section-heading"><h2>Sources</h2></div>
          <div class="claude-settings-card">
            ${claudeSourceControl("claude")}
            ${claudeSourceControl("codex")}
          </div>
        </section>
        ${claudeAppearanceControl()}
        <div class="claude-settings-actions"><button class="claude-save-button" type="button">Save changes</button></div>
      </main>
    </section>`;
}

function variantB() {
  return `
    <section class="settings-window settings-window--b">
      <header class="b-header">
        <div class="settings-brand"><span class="app-mark">T</span><strong>Token Tracing</strong></div>
        <div class="b-header__title"><span>Settings</span><strong>Widget display</strong></div>
        <button class="text-button" type="button">Done</button>
      </header>
      <main class="b-content">
        <section class="b-hero">
          <div><span class="quiet-label">TOTAL TODAY</span><strong>173,816,684</strong><p>Across the providers you have enabled.</p></div>
          <div class="b-hero__providers"><span>${providerDot(providers.claude)}Claude <strong>147.3M</strong></span><span>${providerDot(providers.codex)}Codex <strong>26.5M</strong></span></div>
        </section>
        <section class="b-section">
          <div class="b-section__title"><h1>Show in widget</h1><p>Keep both providers on the canvas for a complete daily picture.</p></div>
          <div class="b-provider-row"><div class="b-provider-row__identity">${providerDot(providers.claude)}<div><strong>Claude Code</strong><span>Idle · last update 3 min ago</span></div></div>${switchControl("claude", "Show in widget", visibility.claude)}</div>
          <div class="b-provider-row"><div class="b-provider-row__identity">${providerDot(providers.codex)}<div><strong>Codex</strong><span>Active · updated just now</span></div></div>${switchControl("codex", "Show in widget", visibility.codex)}</div>
        </section>
        <section class="b-section b-section--sources">
          <div class="b-section__title"><h1>Data sources</h1><p>Local paths are used only by the Rust collector.</p></div>
          <div class="b-source-line"><span>Claude Code</span><span>${providers.claude.root}</span><span class="source-health"><span class="health-dot"></span>Ready</span></div>
          <div class="b-source-line"><span>Codex</span><span>${providers.codex.root}</span><span class="source-health"><span class="health-dot"></span>Ready</span></div>
        </section>
        <div class="b-footer"><span>Local only · No network access</span><button class="primary-button" type="button">Save changes</button></div>
      </main>
    </section>`;
}

function variantC() {
  return `
    <section class="settings-window settings-window--c">
      <header class="c-header">
        <div><h1>Settings</h1><p>Token Tracing</p></div>
        <div class="segmented-control" role="tablist"><button class="is-selected" type="button" role="tab">Widget</button><button type="button" role="tab">Sources</button><button type="button" role="tab">System</button></div>
        <button class="close-button" type="button" aria-label="Close settings">${icon("close")}</button>
      </header>
      <main class="c-content">
        <section class="c-panel c-panel--display">
          <div class="c-panel__header"><div><h2>Widget</h2><p>Choose the information that earns a place in the overlay.</p></div><span class="preview-chip">2 providers</span></div>
          <div class="c-field-label">Visible providers</div>
          ${providerControl("claude", { compact: true, includeRoot: false })}
          ${providerControl("codex", { compact: true, includeRoot: false })}
          <div class="c-total-preview"><span>Always show</span><div><strong>Total today</strong><span>Claude + Codex</span></div><span class="check-mark">${icon("check")}</span></div>
        </section>
        <section class="c-panel c-panel--sources">
          <div class="c-panel__header"><div><h2>Sources</h2><p>Collection is independent from display.</p></div></div>
          <div class="c-source-row"><div>${providerDot(providers.claude)}<div><strong>Claude Code</strong><span>${providers.claude.root}</span></div></div><span class="source-health"><span class="health-dot"></span>Ready</span></div>
          <div class="c-source-row"><div>${providerDot(providers.codex)}<div><strong>Codex</strong><span>${providers.codex.root}</span></div></div><span class="source-health"><span class="health-dot"></span>Ready</span></div>
          <button class="root-button" type="button">Configure source roots</button>
        </section>
      </main>
      <footer class="c-footer"><span>Local only</span><div><button class="secondary-button" type="button">Reset</button><button class="primary-button" type="button">Save changes</button></div></footer>
    </section>`;
}

function renderVariant() {
  const variant = variants.find(({ key }) => key === currentKey);
  const settings = currentKey === "A" ? variantA() : currentKey === "B" ? variantB() : currentKey === "C" ? variantC() : variantD();
  const shellClass = currentKey === "D" ? "review-shell review-shell--claude" : "review-shell";
  const designLabel = currentKey === "D" ? "Claude-inspired settings" : "macOS-inspired settings";
  root.innerHTML = `
    <div class="${shellClass}">
      <header class="review-header">
        <div><span class="review-kicker">Design review · sample data</span><h1>Token Tracing</h1></div>
        <div class="review-header__meta"><span class="preview-chip">${designLabel}</span><span>Settings + overlay</span></div>
      </header>
      <main class="review-stage">
        <section class="settings-column"><div class="stage-label">Settings · ${variant.name}</div>${settings}</section>
        <aside class="overlay-column">${overlayPreview()}</aside>
      </main>
      <footer class="prototype-switcher" aria-label="Design variants">
        <button type="button" data-direction="previous" aria-label="Previous design">${icon("previous")}</button>
        <span><strong>${variant.key}</strong> · ${variant.name}<small>${variant.description}</small></span>
        <button type="button" data-direction="next" aria-label="Next design">${icon("next")}</button>
      </footer>
    </div>`;
  bindPrototypeControls();
}

function setVariant(nextKey) {
  currentKey = nextKey;
  const nextParams = new URLSearchParams(window.location.search);
  nextParams.set("variant", currentKey);
  window.history.replaceState({}, "", `${window.location.pathname}?${nextParams.toString()}`);
  renderVariant();
}

function cycleVariant(direction) {
  const index = variants.findIndex(({ key }) => key === currentKey);
  const nextIndex = (index + direction + variants.length) % variants.length;
  setVariant(variants[nextIndex].key);
}

function bindPrototypeControls() {
  root.querySelectorAll("[data-direction]").forEach((button) => {
    button.addEventListener("click", () => cycleVariant(button.dataset.direction === "next" ? 1 : -1));
  });
  root.querySelectorAll("[data-switch]").forEach((button) => {
    button.addEventListener("click", () => {
      const providerId = button.dataset.switch;
      if (providerId === "dark-mode") {
        darkMode = !darkMode;
        renderVariant();
        return;
      }
      visibility[providerId] = !visibility[providerId];
      renderVariant();
    });
  });
}

window.addEventListener("keydown", (event) => {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
  if (event.key === "ArrowRight") cycleVariant(1);
  if (event.key === "ArrowLeft") cycleVariant(-1);
});

renderVariant();
