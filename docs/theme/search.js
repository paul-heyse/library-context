// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Paul Heyse
// Resolve from this module, so local preview and Pages subpaths use the same artifact.
const bundle = new URL('../pagefind/', import.meta.url);
await import(new URL('pagefind-component-ui.js', bundle));
const { configureInstance, getInstanceManager } = window.PagefindComponents;
configureInstance('default', { bundlePath: bundle.pathname, baseUrl: new URL('../', bundle).pathname });
const instance = getInstanceManager().getInstance('default');
// Set the initial filter when the dialog opens, avoiding index loads on page visits.

const modal = document.createElement('pagefind-modal');
modal.setAttribute('reset-on-close', '');
modal.innerHTML = `
  <pagefind-modal-header><pagefind-input></pagefind-input></pagefind-modal-header>
  <pagefind-modal-body>
    <label class="search-scope">Search in
      <select aria-label="Search scope">
        <option>Current</option><option>Reference</option><option>History</option><option>Everything</option>
      </select>
    </label>
    <pagefind-summary></pagefind-summary><pagefind-results></pagefind-results>
  </pagefind-modal-body>
  <pagefind-modal-footer><pagefind-keyboard-hints></pagefind-keyboard-hints></pagefind-modal-footer>`;
const scope = modal.querySelector('select');
scope.addEventListener('change', () => instance.triggerFilter('scope', scope.value === 'Everything' ? [] : [scope.value]));
// Reset scope whenever the library opens a new dialog, including keyboard opens.
new MutationObserver(() => {
  const dialog = modal.querySelector('dialog');
  if (dialog?.open) {
    scope.value = 'Current';
    instance.triggerFilter('scope', ['Current']);
  }
}).observe(modal, { subtree: true, attributes: true, attributeFilter: ['open'] });
const trigger = document.createElement('pagefind-modal-trigger');
trigger.setAttribute('placeholder', 'Search documentation');
(document.querySelector('.right-buttons') || document.body).prepend(trigger);
document.body.append(modal);
