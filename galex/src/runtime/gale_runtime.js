// GaleX Client Runtime — fine-grained reactive system with SSR hydration.
// Target: <3KB gzipped. No virtual DOM — direct, targeted DOM mutations.

// ── HMR Signal Registry ────────────────────────────────────────────────
// Tracks all active signals by name for hot-reload state preservation.
// The overlay.js saves signal values to sessionStorage before reload;
// we restore them here on the next page load.
let _hmrState = null;
if (typeof window !== 'undefined') {
  window.__gale_signals__ = {};
  try {
    const raw = sessionStorage.getItem('__gale_hmr_state__');
    if (raw) {
      _hmrState = JSON.parse(raw);
      sessionStorage.removeItem('__gale_hmr_state__');
    }
  } catch (_) {}
}

export function _registerSignal(name, sig) {
  if (typeof window !== 'undefined') {
    window.__gale_signals__[name] = sig;
    // Restore preserved value from previous hot reload
    if (_hmrState && name in _hmrState) {
      sig.set(_hmrState[name]);
    }
  }
  return sig;
}

// ── Dependency Tracking ────────────────────────────────────────────────
let _cur = null;
const _stack = [];
function _track(sub) {
  _stack.push(_cur);
  _cur = sub;
  try { sub.run(); }
  finally { _cur = _stack.pop(); }
}

// ── Microtask Scheduler ────────────────────────────────────────────────
let _pending = new Set();
let _batching = false;
let _scheduled = false;

function _schedule(sub) {
  _pending.add(sub);
  if (!_batching && !_scheduled) {
    _scheduled = true;
    queueMicrotask(_flush);
  }
}

function _flush() {
  _scheduled = false;
  const batch = [..._pending];
  _pending.clear();
  for (const sub of batch) _track(sub);
}

// ── signal(initialValue) ───────────────────────────────────────────────
// Reactive mutable container. Reading inside an effect/derive registers
// the subscriber. Writing notifies all subscribers via the scheduler.
export function signal(init) {
  let val = init;
  const subs = new Set();
  return {
    get() {
      if (_cur) subs.add(_cur);
      return val;
    },
    set(v) {
      if (v !== val) {
        val = v;
        for (const s of [...subs]) _schedule(s);
      }
    },
    peek()      { return val; },
    subscribe(fn) {
      const sub = { run: fn };
      subs.add(sub);
      return () => subs.delete(sub);
    }
  };
}

// ── derive(fn) ─────────────────────────────────────────────────────────
// Computed reactive value with auto-dependency tracking and lazy recomputation.
export function derive(fn) {
  let val, dirty = true;
  const subs = new Set();
  const self = {
    run() {
      const prev = val;
      val = fn();
      dirty = false;
      if (val !== prev) {
        for (const s of [...subs]) _schedule(s);
      }
    }
  };
  _track(self);
  return {
    get() {
      if (dirty) _track(self);
      if (_cur) subs.add(_cur);
      return val;
    }
  };
}

// ── effect(fn) ─────────────────────────────────────────────────────────
// Side-effect that re-runs when its reactive dependencies change.
// fn may return a cleanup function called before re-run and on disposal.
export function effect(fn) {
  let cleanup;
  const self = {
    run() {
      if (typeof cleanup === 'function') cleanup();
      cleanup = fn();
    }
  };
  _track(self);
  return () => { if (typeof cleanup === 'function') cleanup(); };
}

// ── watch(sourceFn, callback) ──────────────────────────────────────────
// Watches a reactive expression and calls callback(next, prev) on change.
export function watch(sourceFn, cb) {
  let prev = sourceFn();
  return effect(() => {
    const next = sourceFn();
    if (next !== prev) { cb(next, prev); prev = next; }
    return undefined;
  });
}

// ── batch(fn) ──────────────────────────────────────────────────────────
// Batch multiple signal updates into a single DOM flush.
export function batch(fn) {
  const prev = _batching;
  _batching = true;
  try { fn(); }
  finally {
    _batching = prev;
    if (!prev) _flush();
  }
}

// ── hydrate(instructions) ──────────────────────────────────────────────
// Attach reactivity to server-rendered HTML. Each instruction maps a
// hydration ID to a function that receives the DOM element.
export function hydrate(instructions) {
  for (const [id, fn] of Object.entries(instructions)) {
    const el = document.querySelector(
      `[data-gx-id="${id}"],[data-gx-text="${id}"]`
    );
    if (el) fn(el);
  }
}

// ── bind(el, signal) ───────────────────────────────────────────────────
// Two-way binding between a DOM input element and a signal.
export function bind(el, sig) {
  effect(() => {
    const v = sig.get();
    if (el.type === 'checkbox') {
      el.checked = !!v;
    } else if (el.value !== String(v)) {
      el.value = v;
    }
  });
  const evt = el.type === 'checkbox' ? 'change' : 'input';
  el.addEventListener(evt, () => {
    sig.set(el.type === 'checkbox' ? el.checked : el.value);
  });
}

// ── show(el, condFn) ───────────────────────────────────────────────────
// Conditional display — toggles element visibility based on a reactive condition.
export function show(el, condFn) {
  const orig = el.style.display;
  effect(() => { el.style.display = condFn() ? orig : 'none'; });
}

// ── list(el, itemsFn, keyFn, templateFn) ───────────────────────────────
// Reactive keyed list rendering. Creates/removes/reorders DOM nodes.
export function list(el, itemsFn, keyFn, templateFn) {
  const nodes = new Map();
  effect(() => {
    const items = itemsFn();
    const newKeys = new Set();
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      const key = keyFn ? keyFn(item, i) : i;
      newKeys.add(key);
      if (!nodes.has(key)) {
        const node = templateFn(item, i);
        nodes.set(key, node);
        el.appendChild(node);
      }
    }
    for (const [key, node] of nodes) {
      if (!newKeys.has(key)) {
        node.remove();
        nodes.delete(key);
      }
    }
  });
}

// ── replaceRegion(markerId, renderFn) ──────────────────────────────────
// Replace DOM content between <!--gx-when:N--> and <!--/gx-when:N-->
// comment markers. Used for reactive `when` blocks.
export function replaceRegion(markerId, renderFn) {
  const start = _findComment('gx-when:' + markerId);
  const end = _findComment('/gx-when:' + markerId);
  if (!start || !end) return;
  while (start.nextSibling && start.nextSibling !== end) {
    start.nextSibling.remove();
  }
  const html = renderFn();
  if (html) {
    const tpl = document.createElement('template');
    tpl.innerHTML = html;
    start.parentNode.insertBefore(tpl.content, end);
  }
}

// ── reconcileList(markerId, items, keyFn, renderItem) ──────────────────
// Keyed list reconciliation between <!--gx-each:N--> and <!--/gx-each:N-->
// comment markers. Used for reactive `each` blocks.
export function reconcileList(markerId, items, keyFn, renderItem) {
  const start = _findComment('gx-each:' + markerId);
  const end = _findComment('/gx-each:' + markerId);
  if (!start || !end) return;
  const parent = start.parentNode;
  const existing = new Map();
  let node = start.nextSibling;
  while (node && node !== end) {
    const next = node.nextSibling;
    if (node.__gxKey !== undefined) existing.set(node.__gxKey, node);
    node = next;
  }
  const newNodes = [];
  for (let i = 0; i < items.length; i++) {
    const key = keyFn(items[i], i);
    let el = existing.get(key);
    if (el) { existing.delete(key); }
    else {
      const html = renderItem(items[i], i);
      const tpl = document.createElement('template');
      tpl.innerHTML = html;
      el = tpl.content.firstChild;
    }
    if (el) { el.__gxKey = key; newNodes.push(el); }
  }
  existing.forEach(old => old.remove());
  for (const n of newNodes) parent.insertBefore(n, end);
}

// Helper: find a comment node by text content.
function _findComment(text) {
  const w = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT, null);
  while (w.nextNode()) {
    if (w.currentNode.textContent.trim() === text) return w.currentNode;
  }
  return null;
}

// ── transition(el, type, opts) ─────────────────────────────────────────
// CSS class-based enter/exit transitions. Uses transitionend/animationend
// for completion detection with a timeout fallback.
export function transition(el, type, opts) {
  const dur = (opts && opts.duration) || 300;
  const reduced = typeof matchMedia === 'function'
    && matchMedia('(prefers-reduced-motion: reduce)').matches;
  const d = reduced ? 0 : dur;

  function onEnd(el, cb) {
    if (d === 0) return void cb();
    let done = false;
    const finish = () => { if (!done) { done = true; cb(); } };
    el.addEventListener('transitionend', finish, { once: true });
    el.addEventListener('animationend', finish, { once: true });
    setTimeout(finish, d + 50); // fallback
  }

  return {
    enter() {
      el.classList.add(`gale-${type}-enter`);
      el.offsetHeight; // force reflow
      el.classList.add(`gale-${type}-enter-active`);
      return new Promise(r => onEnd(el, () => {
        el.classList.remove(`gale-${type}-enter`, `gale-${type}-enter-active`);
        r();
      }));
    },
    exit() {
      el.classList.add(`gale-${type}-exit`);
      el.offsetHeight;
      el.classList.add(`gale-${type}-exit-active`);
      return new Promise(r => onEnd(el, () => {
        el.classList.remove(`gale-${type}-exit`, `gale-${type}-exit-active`);
        r();
      }));
    },
  };
}

// ── flipTransition(el) ────────────────────────────────────────────────
// FLIP (First-Last-Invert-Play) animation for list item reordering.
export function flipTransition(el) {
  const first = el.getBoundingClientRect();
  return {
    play() {
      const last = el.getBoundingClientRect();
      const dx = first.left - last.left;
      const dy = first.top - last.top;
      if (dx === 0 && dy === 0) return Promise.resolve();
      el.style.transform = `translate(${dx}px, ${dy}px)`;
      el.offsetHeight;
      el.style.transition = 'transform 0.3s';
      el.style.transform = '';
      return new Promise(r => {
        el.addEventListener('transitionend', () => {
          el.style.transition = '';
          r();
        }, { once: true });
        setTimeout(() => { el.style.transition = ''; r(); }, 350);
      });
    },
  };
}

// ── action(name) ───────────────────────────────────────────────────────
// Returns an async function that POSTs to a GaleX server action endpoint.
export function action(name) {
  return async (data) => {
    const res = await fetch(`/api/__gx/actions/${name}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: data != null ? JSON.stringify(data) : undefined,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw err;
    }
    return res.json();
  };
}

// ── query(urlOrFn, options) ─────────────────────────────────────────────
// Reactive data fetcher. `urlOrFn` can be a string or a function that
// returns a string (enabling reactive URL params via auto-tracking).
// Returns signals for data, loading, error, stale, plus refetch/mutate.
export function query(urlOrFn, opts) {
  const data = signal(null);
  const loading = signal(true);
  const error = signal(null);
  const stale = signal(false);
  let _controller = null;
  const _retries = (opts && opts.retries) || 0;
  const _staleTime = (opts && opts.staleTime) || 0;
  let _lastFetch = 0;

  const load = async () => {
    if (_controller) _controller.abort();
    _controller = new AbortController();
    loading.set(true);
    error.set(null);
    let attempt = 0;
    while (true) {
      try {
        const url = typeof urlOrFn === 'function' ? urlOrFn() : urlOrFn;
        const res = await fetch(url, {
          signal: _controller.signal,
          ...(opts && opts.fetchOpts),
        });
        if (!res.ok) throw new Error(res.statusText);
        data.set(await res.json());
        stale.set(false);
        _lastFetch = Date.now();
        break;
      } catch (e) {
        if (e.name === 'AbortError') return;
        if (attempt < _retries) { attempt++; continue; }
        error.set(e);
        break;
      }
    }
    loading.set(false);
  };

  // Auto-refetch when reactive URL deps change (if urlOrFn is a function)
  if (typeof urlOrFn === 'function') {
    effect(() => {
      urlOrFn(); // track reactive reads
      load();
      return () => { if (_controller) _controller.abort(); };
    });
  } else {
    load();
  }

  // Stale timer — mark data as stale after staleTime ms
  if (_staleTime > 0) {
    effect(() => {
      data.get(); // re-run when data changes
      if (_lastFetch > 0) {
        const id = setTimeout(() => stale.set(true), _staleTime);
        return () => clearTimeout(id);
      }
    });
  }

  return {
    data,
    loading,
    error,
    stale,
    refetch: load,
    mutate(updater) {
      const prev = data.peek();
      data.set(typeof updater === 'function' ? updater(prev) : updater);
      stale.set(true);
    },
  };
}

// ── channel(name, params, opts) ─────────────────────────────────────────
// Reactive WebSocket wrapper for GaleX channels with auto-reconnect.
export function channel(name, params, opts) {
  const qs = params ? new URLSearchParams(params).toString() : '';
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const url = `${proto}//${location.host}/ws/__gx/channels/${name}${qs ? '?' + qs : ''}`;
  const messages = signal([]);
  const connected = signal(false);
  const _maxRetries = (opts && opts.maxRetries != null) ? opts.maxRetries : 5;
  let ws, _retries = 0, _reconnectTimer = null, _disposed = false;

  function connect() {
    if (_disposed) return;
    ws = new WebSocket(url);
    ws.onopen = () => { connected.set(true); _retries = 0; };
    ws.onclose = () => {
      connected.set(false);
      // Auto-reconnect with exponential backoff
      if (!_disposed && _retries < _maxRetries) {
        const delay = Math.min(1000 * (2 ** _retries), 30000);
        _reconnectTimer = setTimeout(() => { _retries++; connect(); }, delay);
      }
    };
    ws.onmessage = e => {
      let msg;
      try { msg = JSON.parse(e.data); } catch (_) { msg = e.data; }
      messages.set([...messages.peek(), msg]);
    };
  }
  connect();

  return {
    messages,
    connected,
    send(d) { if (ws && ws.readyState === 1) ws.send(JSON.stringify(d)); },
    close() { if (ws) ws.close(); },
    reconnect() { _retries = 0; connect(); },
    dispose() {
      _disposed = true;
      clearTimeout(_reconnectTimer);
      if (ws) ws.close();
    },
  };
}

// ── navigate(url, options) ─────────────────────────────────────────────
// Client-side navigation via History API.
export function navigate(url, opts) {
  if (opts && opts.replace) {
    history.replaceState(null, '', url);
  } else {
    history.pushState(null, '', url);
  }
  window.dispatchEvent(new PopStateEvent('popstate'));
}

// ── Data readers ───────────────────────────────────────────────────────
// Read embedded server data from the page.
export function _readData() {
  const el = document.querySelector('script[type="gale-data"]');
  return el ? JSON.parse(el.textContent) : {};
}

export function _readEnv() {
  const el = document.querySelector('script[type="gale-env"]');
  return el ? JSON.parse(el.textContent) : {};
}

// ── Error Classes ──────────────────────────────────────────────────────
// Used by action stubs for typed error handling.

export class GaleValidationError extends Error {
  constructor(action, errors) {
    super(`Validation failed for action '${action}'`);
    this.name = 'GaleValidationError';
    this.action = action;
    this.errors = errors;
  }
}

export class GaleServerError extends Error {
  constructor(action, status, body) {
    super(`Server error ${status} for action '${action}'`);
    this.name = 'GaleServerError';
    this.action = action;
    this.status = status;
    this.body = body;
  }
}

export class GaleNetworkError extends Error {
  constructor(action, cause) {
    super(`Network error for action '${action}'`);
    this.name = 'GaleNetworkError';
    this.action = action;
    this.cause = cause;
  }
}

// ── __gx_fetch(actionName, body) ───────────────────────────────────────
// Internal POST helper for action endpoints with error classification.
export async function __gx_fetch(actionName, body) {
  let response;
  try {
    response = await fetch(`/api/__gx/actions/${actionName}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  } catch (err) {
    throw new GaleNetworkError(actionName, err);
  }
  if (!response.ok) {
    let responseBody;
    try { responseBody = await response.json(); } catch (_) { responseBody = null; }
    if (response.status === 400 && responseBody?.error === 'validation_failed') {
      throw new GaleValidationError(actionName, responseBody.details);
    }
    throw new GaleServerError(actionName, response.status, responseBody);
  }
  return response.json();
}

// ── GaleQueryCache ─────────────────────────────────────────────────────
// Reactive query cache supporting optimistic updates, rollback, and
// invalidation (re-fetch). Used by action `.withMutate()` helpers.

class GaleQueryCache {
  constructor() {
    this._cache = new Map();
    this._subscribers = new Map();
    this._fetchers = new Map();
  }

  register(name, fetcher) {
    this._fetchers.set(name, fetcher);
  }

  fetch(name, ...args) {
    const fetcher = this._fetchers.get(name);
    if (!fetcher) throw new Error(`Unknown query: ${name}`);
    const key = `${name}:${JSON.stringify(args)}`;
    return fetcher(...args).then(data => {
      this._cache.set(key, { data, timestamp: Date.now() });
      this._notify(name, key, data);
      return data;
    });
  }

  get(name, ...args) {
    const key = `${name}:${JSON.stringify(args)}`;
    const entry = this._cache.get(key);
    return entry ? entry.data : null;
  }

  subscribe(name, callback) {
    if (!this._subscribers.has(name)) {
      this._subscribers.set(name, new Set());
    }
    this._subscribers.get(name).add(callback);
    return () => {
      const set = this._subscribers.get(name);
      if (set) set.delete(callback);
    };
  }

  mutate(name, updater) {
    const prefix = `${name}:`;
    for (const [key, val] of this._cache) {
      if (key.startsWith(prefix)) {
        val._rollback = val.data;
        val.data = typeof updater === 'function' ? updater(val.data) : updater;
        this._notify(name, key, val.data);
      }
    }
  }

  rollback(name) {
    const prefix = `${name}:`;
    for (const [key, val] of this._cache) {
      if (key.startsWith(prefix) && val._rollback !== undefined) {
        val.data = val._rollback;
        delete val._rollback;
        this._notify(name, key, val.data);
      }
    }
  }

  invalidate(name) {
    const prefix = `${name}:`;
    for (const [key] of this._cache) {
      if (key.startsWith(prefix)) {
        const argsJson = key.slice(prefix.length);
        let args;
        try { args = JSON.parse(argsJson); } catch (_) { args = []; }
        this.fetch(name, ...args);
      }
    }
  }

  _notify(name, _key, data) {
    const subs = this._subscribers.get(name);
    if (subs) subs.forEach(cb => cb(data));
  }
}

export const queryCache = new GaleQueryCache();
