// GaleX Dev Client — injected by `gale dev` into every page.
// Handles: WebSocket connection, page reload, CSS hot reload, error overlay.
(function() {
  'use strict';

  var ws = null;
  var overlay = null;
  var reconnectTimer = null;

  function connect() {
    ws = new WebSocket('ws://' + location.host + '/__gale_dev/ws');

    ws.onmessage = function(e) {
      var msg;
      try { msg = JSON.parse(e.data); } catch (_) { return; }

      switch (msg.type) {
        case 'Reload':
          location.reload();
          break;

        case 'CssReload':
          // Reload CSS without page reload — cache-bust with timestamp
          var links = document.querySelectorAll('link[rel="stylesheet"]');
          for (var i = 0; i < links.length; i++) {
            var href = links[i].href.split('?')[0];
            if (href.indexOf('/_gale/') !== -1) {
              links[i].href = href + '?t=' + Date.now();
            }
          }
          break;

        case 'AssetReload':
          // Reload specific asset (images, etc.)
          var imgs = document.querySelectorAll('img[src*="' + msg.path + '"]');
          for (var j = 0; j < imgs.length; j++) {
            imgs[j].src = imgs[j].src.split('?')[0] + '?t=' + Date.now();
          }
          break;

        case 'Error':
          showOverlay(msg.errors || []);
          break;

        case 'ErrorCleared':
          hideOverlay();
          break;
      }
    };

    ws.onclose = function() {
      showReconnecting();
      // Retry connection every 2 seconds
      clearInterval(reconnectTimer);
      reconnectTimer = setInterval(function() {
        try {
          var retry = new WebSocket('ws://' + location.host + '/__gale_dev/ws');
          retry.onopen = function() {
            clearInterval(reconnectTimer);
            hideReconnecting();
            location.reload();
          };
          retry.onerror = function() { retry.close(); };
        } catch (_) {}
      }, 2000);
    };
  }

  // ── Error Overlay ──────────────────────────────────────────────────

  function showOverlay(errors) {
    hideOverlay();
    overlay = document.createElement('div');
    overlay.id = 'gale-dev-overlay';

    var html = '<div class="gale-overlay-header">';
    html += '<span>Found ' + errors.length + ' error' + (errors.length !== 1 ? 's' : '') + '</span>';
    html += '<button class="gale-overlay-close" onclick="document.getElementById(\'gale-dev-overlay\').remove()">&times;</button>';
    html += '</div>';
    html += '<div class="gale-overlay-body">';

    for (var i = 0; i < errors.length; i++) {
      var err = errors[i];
      html += '<div class="gale-error-card">';
      if (err.file) {
        html += '<div class="gale-error-file">' + escapeHtml(err.file);
        if (err.line > 0) html += ':' + err.line + ':' + err.col;
        html += '</div>';
      }
      html += '<div class="gale-error-message">';
      if (err.code) html += '<span class="gale-error-code">' + escapeHtml(err.code) + '</span> ';
      html += escapeHtml(err.message);
      html += '</div>';
      if (err.source_line) {
        html += '<pre class="gale-error-source"><code>' + escapeHtml(err.source_line) + '</code></pre>';
      }
      if (err.suggestion) {
        html += '<div class="gale-error-suggestion">' + escapeHtml(err.suggestion) + '</div>';
      }
      html += '</div>';
    }

    html += '</div>';
    overlay.innerHTML = html;
    document.body.appendChild(overlay);
  }

  function hideOverlay() {
    if (overlay && overlay.parentNode) {
      overlay.parentNode.removeChild(overlay);
    }
    overlay = null;
  }

  // ── Reconnecting Banner ────────────────────────────────────────────

  function showReconnecting() {
    if (document.getElementById('gale-dev-reconnecting')) return;
    var banner = document.createElement('div');
    banner.id = 'gale-dev-reconnecting';
    banner.textContent = 'Server disconnected — reconnecting...';
    document.body.appendChild(banner);
  }

  function hideReconnecting() {
    var banner = document.getElementById('gale-dev-reconnecting');
    if (banner) banner.parentNode.removeChild(banner);
  }

  // ── Utilities ──────────────────────────────────────────────────────

  function escapeHtml(text) {
    var div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // ── Initialize ─────────────────────────────────────────────────────
  connect();
})();
