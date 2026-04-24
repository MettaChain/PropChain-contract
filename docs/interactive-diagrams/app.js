/* PropChain Interactive Architecture Explorer — app.js */
(function () {
  'use strict';

  /* ── state ── */
  const S = {
    diagrams: [], index: {}, currentId: null,
    zoom: 1, panX: 0, panY: 0, dragging: false, dragStart: { x: 0, y: 0 },
    stepMode: false, stepIndex: 0, stepTotal: 0, playTimer: null,
    participantIndex: {}
  };

  /* ── refs ── */
  const $ = id => document.getElementById(id);
  const canvas = $('canvas'), canvasWrap = $('canvas-wrap'),
    sidebar = $('sidebar'), tooltip = $('tooltip'),
    infoPanel = $('info-panel'), searchInput = $('search'),
    stepBar = $('step-bar'), minimap = $('minimap');

  /* ── mermaid init ── */
  mermaid.initialize({
    startOnLoad: false, theme: 'dark', fontFamily: 'Inter,system-ui,sans-serif',
    themeVariables: {
      darkMode: true, background: '#0a0e1a',
      primaryColor: '#1e293b', primaryTextColor: '#e2e8f0', primaryBorderColor: '#38bdf8',
      secondaryColor: '#312e81', secondaryTextColor: '#e2e8f0', secondaryBorderColor: '#a78bfa',
      tertiaryColor: '#7c2d12', tertiaryTextColor: '#e2e8f0', tertiaryBorderColor: '#fb923c',
      lineColor: '#475569', textColor: '#e2e8f0', mainBkg: '#1e293b', nodeBorder: '#38bdf8',
      actorBkg: '#1e293b', actorBorder: '#38bdf8', actorTextColor: '#e2e8f0', actorLineColor: '#475569',
      signalColor: '#94a3b8', signalTextColor: '#e2e8f0',
      noteBkgColor: '#312e81', noteTextColor: '#e2e8f0', noteBorderColor: '#a78bfa',
      activationBkgColor: '#1e3a5f', activationBorderColor: '#38bdf8',
      labelBoxBkgColor: '#1e293b', labelBoxBorderColor: '#38bdf8', labelTextColor: '#38bdf8',
      loopTextColor: '#a78bfa',
    },
    sequence: { actorMargin: 80, mirrorActors: true, wrap: true, width: 200 }
  });

  /* ── slugify ── */
  function slug(t) {
    return t.replace(/^\d+\.\s*/, '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '');
  }

  /* ── parse markdown ── */
  function parseMD(md, filename) {
    const lines = md.split(/\r?\n/);
    let cat = '', title = '', out = [];
    for (let i = 0; i < lines.length; i++) {
      const l = lines[i];
      if (/^## /.test(l)) cat = l.slice(3).trim();
      if (/^### /.test(l)) title = l.slice(4).trim();
      if (l.trim() === '```mermaid') {
        let code = ''; i++;
        while (i < lines.length && lines[i].trim() !== '```') { code += lines[i] + '\n'; i++; }
        const c = code.trim(), type = c.startsWith('stateDiagram') ? 'state' : 'sequence';
        const id = slug(title || 'diagram-' + out.length);
        out.push({ id, title: title || 'Diagram ' + out.length, category: cat || 'General', type, code: c, source: filename });
      }
    }
    return out;
  }

  /* ── category config ── */
  const CAT_ICONS = {
    'Core Property Lifecycle': '🏠', 'Trading & Transfer Operations': '💱',
    'Compliance & Verification': '✅', 'Cross-Chain Operations': '🌉',
    'Insurance & Risk Management': '🛡️', 'Oracle & Valuation': '📊',
    'Governance & Administration': '⚖️', 'Error Handling & Edge Cases': '⚠️',
    'State Machine Diagrams': '🔄', 'Deployment Sequence Diagrams': '🚀',
    'Quality Standards': '📝', 'Diagram Standards': '📝', 'General': '📄'
  };

  /* ── build sidebar ── */
  function buildSidebar(diagrams, filter) {
    const cats = {};
    diagrams.forEach(d => {
      if (filter) {
        const q = filter.toLowerCase();
        if (!d.title.toLowerCase().includes(q) && !d.category.toLowerCase().includes(q) && !d.code.toLowerCase().includes(q)) return;
      }
      (cats[d.category] = cats[d.category] || []).push(d);
    });
    let html = '';
    Object.entries(cats).forEach(([cat, items]) => {
      const icon = CAT_ICONS[cat] || '📄';
      html += `<div class="cat-group"><div class="cat-header" data-cat="${cat}"><span class="arrow">▾</span>${icon} ${cat}</div><div class="cat-items">`;
      items.forEach(d => {
        const badge = d.type === 'state' ? '<span class="badge badge-state">STATE</span>' : '<span class="badge badge-seq">SEQ</span>';
        const active = d.id === S.currentId ? ' active' : '';
        html += `<div class="dia-item${active}" data-id="${d.id}">${d.title.replace(/^\d+\.\s*/, '')} ${badge}</div>`;
      });
      html += '</div></div>';
    });
    sidebar.innerHTML = html || '<div style="padding:20px;color:var(--t3);font-size:13px">No diagrams match your search.</div>';
    sidebar.querySelectorAll('.cat-header').forEach(h => h.addEventListener('click', () => h.classList.toggle('collapsed')));
    sidebar.querySelectorAll('.dia-item').forEach(el => el.addEventListener('click', () => selectDiagram(el.dataset.id)));
  }

  /* ── build participant cross-reference index ── */
  function buildParticipantIndex() {
    S.participantIndex = {};
    S.diagrams.forEach(d => {
      const matches = d.code.matchAll(/participant\s+(\w+)(?:\s+as\s+(.+))?/g);
      for (const m of matches) {
        const name = (m[2] || m[1]).trim();
        (S.participantIndex[name] = S.participantIndex[name] || new Set()).add(d.id);
      }
    });
  }

  /* ── select & render diagram ── */
  let renderCounter = 0;
  async function selectDiagram(id) {
    const d = S.index[id]; if (!d) return;
    S.currentId = id;
    resetView();
    exitStepMode();
    $('diagram-title').textContent = d.title;
    $('diagram-meta').innerHTML = `<span>📁 ${d.source}</span><span>🏷 ${d.type}</span><span>📂 ${d.category}</span>`;
    sidebar.querySelectorAll('.dia-item').forEach(el => el.classList.toggle('active', el.dataset.id === id));
    canvas.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--t3)"><div class="spinner" style="width:28px;height:28px;border:2px solid var(--bg4);border-top-color:var(--cyan);border-radius:50%;animation:spin .8s linear infinite"></div></div>';
    const rid = 'mrender-' + (++renderCounter);
    try {
      const { svg } = await mermaid.render(rid, d.code);
      canvas.innerHTML = svg;
      canvas.style.opacity = '0';
      requestAnimationFrame(() => { canvas.style.transition = 'opacity .35s'; canvas.style.opacity = '1'; });
      attachSVGHandlers(d);
      updateMinimap();
    } catch (e) {
      canvas.innerHTML = `<div style="padding:40px;color:var(--red)"><h3>Render Error</h3><pre style="margin-top:12px;font-size:12px;color:var(--t3);white-space:pre-wrap">${e.message || e}</pre><pre style="margin-top:16px;font-size:11px;color:var(--t3);max-height:200px;overflow:auto">${d.code}</pre></div>`;
    }
    // Update URL
    history.replaceState(null, '', '?diagram=' + id);
  }

  /* ── attach SVG interactivity ── */
  function attachSVGHandlers(diagram) {
    const svg = canvas.querySelector('svg'); if (!svg) return;
    // Actors (sequence diagrams)
    svg.querySelectorAll('.actor').forEach(el => {
      el.style.cursor = 'pointer';
      el.addEventListener('mouseenter', e => showTooltip(e, el.textContent.trim()));
      el.addEventListener('mouseleave', hideTooltip);
      el.addEventListener('click', e => { e.stopPropagation(); showInfo(el.textContent.trim(), 'Actor', diagram); });
    });
    // State nodes
    svg.querySelectorAll('.statediagram-state .state-id, .statediagram-state text').forEach(el => {
      el.style.cursor = 'pointer';
      const parent = el.closest('.statediagram-state') || el;
      parent.addEventListener('mouseenter', e => showTooltip(e, el.textContent.trim()));
      parent.addEventListener('mouseleave', hideTooltip);
      parent.addEventListener('click', e => { e.stopPropagation(); showInfo(el.textContent.trim(), 'State', diagram); });
    });
    // Messages
    svg.querySelectorAll('.messageText').forEach(el => {
      el.addEventListener('mouseenter', e => showTooltip(e, el.textContent.trim()));
      el.addEventListener('mouseleave', hideTooltip);
    });
    // Notes
    svg.querySelectorAll('.note').forEach(el => {
      el.addEventListener('mouseenter', e => {
        const txt = el.querySelector('text'); if (txt) showTooltip(e, txt.textContent.trim());
      });
      el.addEventListener('mouseleave', hideTooltip);
    });
    // Click canvas to close info
    svg.addEventListener('click', () => closeInfo());
  }

  /* ── tooltip ── */
  function showTooltip(e, text) {
    tooltip.textContent = text;
    tooltip.style.left = e.clientX + 12 + 'px';
    tooltip.style.top = e.clientY - 8 + 'px';
    tooltip.classList.add('visible');
  }
  function hideTooltip() { tooltip.classList.remove('visible'); }

  /* ── info panel ── */
  function showInfo(name, type, diagram) {
    $('info-node-name').textContent = name;
    $('info-type').textContent = type + ' — ' + diagram.category;
    // Find connections
    const conns = [];
    const lines = diagram.code.split('\n');
    lines.forEach(l => {
      if (l.includes(name) && (l.includes('->>') || l.includes('-->>') || l.includes('-->') || l.includes('--->'))) {
        const msg = l.replace(/.*:\s*/, '').trim();
        if (msg) conns.push(msg);
      }
    });
    $('info-connections').innerHTML = conns.length ? conns.map(c => `<li>${c}</li>`).join('') : '<li style="color:var(--t3)">No direct messages</li>';
    // Cross-references
    const xrefs = [];
    Object.entries(S.participantIndex).forEach(([pName, ids]) => {
      if (pName.includes(name) || name.includes(pName)) {
        ids.forEach(did => { if (did !== diagram.id) xrefs.push(did); });
      }
    });
    const unique = [...new Set(xrefs)];
    $('info-xrefs').innerHTML = unique.length
      ? unique.map(did => `<li><span class="xref-link" data-id="${did}">${S.index[did]?.title || did}</span></li>`).join('')
      : '<li style="color:var(--t3)">Only in this diagram</li>';
    $('info-xrefs').querySelectorAll('.xref-link').forEach(el => el.addEventListener('click', () => selectDiagram(el.dataset.id)));
    infoPanel.classList.add('open');
  }
  function closeInfo() { infoPanel.classList.remove('open'); }

  /* ── zoom & pan ── */
  function applyTransform() {
    canvas.style.transform = `translate(${S.panX}px,${S.panY}px) scale(${S.zoom})`;
    $('zoom-level').textContent = Math.round(S.zoom * 100) + '%';
    updateMinimap();
  }
  function resetView() { S.zoom = 1; S.panX = 0; S.panY = 0; applyTransform(); }

  canvasWrap.addEventListener('wheel', e => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    const newZoom = Math.max(0.2, Math.min(5, S.zoom * delta));
    const rect = canvasWrap.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    S.panX = mx - (mx - S.panX) * (newZoom / S.zoom);
    S.panY = my - (my - S.panY) * (newZoom / S.zoom);
    S.zoom = newZoom;
    applyTransform();
  }, { passive: false });

  canvasWrap.addEventListener('mousedown', e => { if (e.button !== 0) return; S.dragging = true; S.dragStart = { x: e.clientX - S.panX, y: e.clientY - S.panY }; });
  window.addEventListener('mousemove', e => { if (!S.dragging) return; S.panX = e.clientX - S.dragStart.x; S.panY = e.clientY - S.dragStart.y; applyTransform(); });
  window.addEventListener('mouseup', () => { S.dragging = false; });

  $('zoom-in').addEventListener('click', () => { S.zoom = Math.min(5, S.zoom * 1.2); applyTransform(); });
  $('zoom-out').addEventListener('click', () => { S.zoom = Math.max(0.2, S.zoom / 1.2); applyTransform(); });
  $('zoom-reset').addEventListener('click', resetView);

  /* ── minimap ── */
  function updateMinimap() {
    const svg = canvas.querySelector('svg');
    if (!svg) { minimap.innerHTML = ''; return; }
    const clone = svg.cloneNode(true);
    clone.setAttribute('width', '100%'); clone.setAttribute('height', '100%');
    const wrapRect = canvasWrap.getBoundingClientRect();
    const svgW = svg.getBoundingClientRect().width * S.zoom;
    const svgH = svg.getBoundingClientRect().height * S.zoom;
    const vw = Math.min(100, (wrapRect.width / svgW) * 100);
    const vh = Math.min(100, (wrapRect.height / svgH) * 100);
    const vx = Math.max(0, (-S.panX / svgW) * 100);
    const vy = Math.max(0, (-S.panY / svgH) * 100);
    minimap.innerHTML = '';
    minimap.appendChild(clone);
    const vp = document.createElement('div');
    vp.className = 'viewport';
    vp.style.cssText = `left:${vx}%;top:${vy}%;width:${vw}%;height:${vh}%`;
    minimap.appendChild(vp);
  }

  /* ── step-through mode ── */
  function enterStepMode() {
    const d = S.index[S.currentId]; if (!d || d.type !== 'sequence') return;
    S.stepMode = true;
    const msgs = canvas.querySelectorAll('.messageLine0, .messageLine1, .messageText');
    S.stepTotal = canvas.querySelectorAll('.messageText').length;
    S.stepIndex = 0;
    msgs.forEach(el => { el.style.opacity = '0'; el.style.transition = 'opacity .3s'; });
    // Also hide activations
    canvas.querySelectorAll('[class*="activation"]').forEach(el => { el.style.opacity = '0'; el.style.transition = 'opacity .3s'; });
    stepBar.classList.add('visible');
    updateStepDisplay();
  }
  function exitStepMode() {
    S.stepMode = false; S.stepIndex = 0;
    if (S.playTimer) { clearInterval(S.playTimer); S.playTimer = null; }
    stepBar.classList.remove('visible');
    canvas.querySelectorAll('.messageLine0,.messageLine1,.messageText,[class*="activation"]').forEach(el => { el.style.opacity = '1'; });
    $('step-play').textContent = '▶';
  }
  function stepTo(n) {
    S.stepIndex = Math.max(0, Math.min(S.stepTotal, n));
    const textEls = canvas.querySelectorAll('.messageText');
    const lineEls = canvas.querySelectorAll('.messageLine0, .messageLine1');
    // Pair lines with texts (mermaid generates 1 line per message approx)
    textEls.forEach((el, i) => { el.style.opacity = i < S.stepIndex ? '1' : '0'; });
    // Show corresponding lines
    const linesPerMsg = Math.max(1, Math.floor(lineEls.length / Math.max(1, textEls.length)));
    lineEls.forEach((el, i) => { el.style.opacity = Math.floor(i / linesPerMsg) < S.stepIndex ? '1' : '0'; });
    updateStepDisplay();
  }
  function updateStepDisplay() { $('step-info').textContent = S.stepIndex + ' / ' + S.stepTotal; }

  $('step-next').addEventListener('click', () => stepTo(S.stepIndex + 1));
  $('step-prev').addEventListener('click', () => stepTo(S.stepIndex - 1));
  $('step-play').addEventListener('click', () => {
    if (S.playTimer) { clearInterval(S.playTimer); S.playTimer = null; $('step-play').textContent = '▶'; return; }
    $('step-play').textContent = '⏸';
    S.playTimer = setInterval(() => {
      if (S.stepIndex >= S.stepTotal) { clearInterval(S.playTimer); S.playTimer = null; $('step-play').textContent = '▶'; return; }
      stepTo(S.stepIndex + 1);
    }, 800);
  });
  $('btn-step').addEventListener('click', () => { if (S.stepMode) exitStepMode(); else enterStepMode(); });

  /* ── search ── */
  searchInput.addEventListener('input', () => buildSidebar(S.diagrams, searchInput.value));

  /* ── fullscreen ── */
  $('btn-fullscreen').addEventListener('click', () => $('app').classList.toggle('fullscreen'));

  /* ── sidebar toggle ── */
  $('toggle-sidebar').addEventListener('click', () => $('app').classList.toggle('sidebar-closed'));

  /* ── export ── */
  $('btn-export').addEventListener('click', () => $('export-modal').classList.add('open'));
  $('export-modal').addEventListener('click', e => { if (e.target === $('export-modal')) $('export-modal').classList.remove('open'); });
  $('export-svg').addEventListener('click', () => {
    const svg = canvas.querySelector('svg'); if (!svg) return;
    const blob = new Blob([new XMLSerializer().serializeToString(svg)], { type: 'image/svg+xml' });
    dl(URL.createObjectURL(blob), (S.currentId || 'diagram') + '.svg');
    $('export-modal').classList.remove('open');
  });
  $('export-png').addEventListener('click', () => {
    const svg = canvas.querySelector('svg'); if (!svg) return;
    const svgData = new XMLSerializer().serializeToString(svg);
    const img = new Image();
    const blob = new Blob([svgData], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    img.onload = () => {
      const c = document.createElement('canvas');
      c.width = img.naturalWidth * 2; c.height = img.naturalHeight * 2;
      const ctx = c.getContext('2d'); ctx.scale(2, 2); ctx.drawImage(img, 0, 0);
      c.toBlob(b => { dl(URL.createObjectURL(b), (S.currentId || 'diagram') + '.png'); URL.revokeObjectURL(url); }, 'image/png');
    };
    img.src = url;
    $('export-modal').classList.remove('open');
  });
  function dl(href, name) { const a = document.createElement('a'); a.href = href; a.download = name; a.click(); }

  /* ── close info ── */
  $('close-info').addEventListener('click', closeInfo);

  /* ── keyboard shortcuts ── */
  document.addEventListener('keydown', e => {
    if (e.target.tagName === 'INPUT') return;
    switch (e.key) {
      case 'Escape': closeInfo(); $('app').classList.remove('fullscreen'); $('export-modal').classList.remove('open'); break;
      case 'f': case 'F': $('app').classList.toggle('fullscreen'); break;
      case '+': case '=': S.zoom = Math.min(5, S.zoom * 1.2); applyTransform(); break;
      case '-': S.zoom = Math.max(0.2, S.zoom / 1.2); applyTransform(); break;
      case '0': resetView(); break;
      case ' ':
        if (S.stepMode) { e.preventDefault(); stepTo(S.stepIndex + 1); } break;
      case 'ArrowDown': case 'ArrowRight': e.preventDefault(); navDiagram(1); break;
      case 'ArrowUp': case 'ArrowLeft': e.preventDefault(); navDiagram(-1); break;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') { e.preventDefault(); searchInput.focus(); }
  });
  function navDiagram(dir) {
    const idx = S.diagrams.findIndex(d => d.id === S.currentId);
    const next = S.diagrams[idx + dir];
    if (next) selectDiagram(next.id);
  }

  /* ── fetch & init ── */
  async function init() {
    const files = [
      { path: '../COMPONENT_INTERACTION_DIAGRAMS.md', name: 'COMPONENT_INTERACTION_DIAGRAMS.md' },
      { path: '../ARCHITECTURE_DOCUMENTATION_MAINTENANCE.md', name: 'ARCHITECTURE_DOCUMENTATION_MAINTENANCE.md' }
    ];
    let allDiagrams = [];
    for (const f of files) {
      try {
        const resp = await fetch(f.path);
        if (!resp.ok) throw new Error(resp.status);
        const text = await resp.text();
        allDiagrams = allDiagrams.concat(parseMD(text, f.name));
      } catch (e) {
        console.warn('Could not fetch ' + f.path + ':', e.message);
      }
    }
    if (allDiagrams.length === 0) {
      canvas.innerHTML = `<div style="padding:40px;text-align:center;color:var(--t3)">
        <h2 style="color:var(--red);margin-bottom:12px">Could not load diagrams</h2>
        <p>This explorer needs to be served via HTTP. Run:</p>
        <pre style="margin:16px auto;padding:12px;background:var(--bg3);border-radius:8px;display:inline-block;text-align:left;color:var(--cyan);font-family:'JetBrains Mono',monospace;font-size:13px">cd docs\nnpx -y serve .</pre>
        <p style="margin-top:12px">Then open <code>http://localhost:3000/interactive-diagrams/</code></p></div>`;
      $('loading').classList.add('hidden');
      return;
    }
    S.diagrams = allDiagrams;
    allDiagrams.forEach(d => S.index[d.id] = d);
    buildParticipantIndex();
    buildSidebar(S.diagrams);
    // Deep link
    const params = new URLSearchParams(location.search);
    const target = params.get('diagram');
    if (target && S.index[target]) selectDiagram(target);
    else if (allDiagrams.length) selectDiagram(allDiagrams[0].id);
    $('loading').classList.add('hidden');
  }

  init();
})();
