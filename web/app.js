'use strict';
(() => {
  const data = globalThis.GHOSTLINE_CATALOGUE;
  const $ = id => document.getElementById(id);
  if (!data) { $('results').textContent = 'Catalogue data could not be loaded.'; return; }
  const fmt = value => value.toLocaleString();
  const indexes = new Map();
  const choices = new Map(data.choices.map(v => [v.id, v]));
  const controls = new Map(data.controls.map(v => [v.id, v]));
  data.items.sort((a,b) => Number(a.label === a.native_name) - Number(b.label === b.native_name) || a.label.localeCompare(b.label));
  const groups = {items: data.items, clothing: Object.values(data.assets).filter(v => v.roles?.includes('clothing')), props: data.props ?? [], controls: data.controls, choices: data.choices, assets: Object.values(data.assets)};
  groups.props.sort((a,b) => Number(a.kind !== 'assembly') - Number(b.kind !== 'assembly') || a.label.localeCompare(b.label));
  const resourceText = r => r?.Path ?? r?.Hash ?? '';
  const label = v => v.label ?? v.native?.name ?? (resourceText(v.resource) || v.id);
  const category = v => v.category ?? v.native?.ui_slot ?? controls.get(v.control)?.native.ui_slot ?? v.roles?.[0] ?? 'Unclassified';
  const technical = v => v.native_name ?? (resourceText(v.resource) || v.native_id || v.id);
  const selectionIds = v => v.selections ?? (v.selection ? [v.selection] : Object.hasOwn(data.selections, v.id) ? [v.id] : []);
  const previews = v => selectionIds(v).flatMap(id => (data.selections[id]?.media ?? []).map(media => ({...media, appearance: data.selections[id].appearance})));
  const mediaURL = path => {
    const base = globalThis.GHOSTLINE_MEDIA_BASE || '';
    return base ? base.replace(/\/$/, '') + '/' + path : path;
  };
  for (const list of [...Object.values(groups), Object.values(data.selections)]) for (const v of list) indexes.set(v.id, v);
  let page = 0, result = [], lastBrowse = '#';
  const pageSize = 30;
  $('coverage').textContent = `${fmt(data.items.length)} records \u00b7 ${fmt(groups.props.length)} props \u00b7 ${fmt(data.controls.length)} controls \u00b7 ${fmt(data.choices.length)} choices \u00b7 ${fmt(Object.keys(data.assets).length)} resources`;
  $('source-version').textContent = 'Cyberpunk 2077 ' + (data.game_version || '');
  function el(tag, text, cls) { const e = document.createElement(tag); if (text != null) e.textContent = text; if (cls) e.className = cls; return e; }
  function link(id, text) { const a = el('a', text); a.href = '#entry/' + encodeURIComponent(id); return a; }
  function filterOptions() {
    $('prop-kind-label').hidden = $('collection').value !== 'props';
    const prior = $('category').value;
    $('category').replaceChildren(new Option('All', ''));
    const values = [...new Set(groups[$('collection').value].map(category))].sort();
    for (const value of values) $('category').add(new Option(value, value));
    if (values.includes(prior)) $('category').value = prior;
  }
  function search() {
    const terms = $('search').value.toLowerCase().trim().split(/\s+/).filter(Boolean), cat = $('category').value;
    result = groups[$('collection').value].filter(v => ($('collection').value !== 'props' || !$('prop-kind').value || v.kind === $('prop-kind').value) && (!cat || category(v) === cat) && (!$('ready').checked || previews(v).length) && terms.every(t => {
      // Include resolved component paths in searches for item mesh identities.
      const selections = selectionIds(v).map(id => data.selections[id]);
      return JSON.stringify([v, category(v), ...(v.control ? selections : [])]).toLowerCase().includes(t);
    }));
    page = 0; draw();
  }
  function draw() {
    const nodes = result.slice(page * pageSize, (page + 1) * pageSize).map(v => {
      const card = link(v.id, null); card.className = 'card';
      const visual = el('div', null, 'visual'), images = previews(v), hero = images.find(m => m.view === 'hero') ?? images[0];
      if (hero) { const img = el('img'); img.src = mediaURL(hero.path); img.alt = `${label(v)} \u2014 ${hero.view}`; img.loading = 'lazy'; visual.append(img); }
      else visual.append(el('span', selectionIds(v).length ? 'Preview ' + (data.selections[selectionIds(v)[0]]?.preview_state ?? 'pending') : 'Native metadata', 'placeholder'));
      const body = el('div', null, 'card-body'); body.append(el('span', category(v), 'tag'), el('h2', label(v)), el('div', technical(v), 'native'));
      if (v.kind === 'assembly' || v.kind === 'mesh') body.prepend(el('span', v.kind === 'assembly' ? 'Complete assembly' : 'Individual mesh', 'tag'));
      card.append(visual, body); return card;
    });
    $('results').replaceChildren(...(nodes.length ? nodes : [el('p', 'No matching entries. Try another collection or a broader search.', 'empty')]));
    $('result-count').textContent = `${fmt(result.length)} entries`;
    $('page').textContent = `${result.length ? page + 1 : 0} / ${Math.ceil(result.length / pageSize)}`;
    $('previous').disabled = page === 0; $('next').disabled = (page + 1) * pageSize >= result.length;
  }
  function browseHash() {
    const p = new URLSearchParams({collection: $('collection').value, q: $('search').value, category: $('category').value, representation: $('prop-kind').value, ready: $('ready').checked ? '1' : ''});
    return '#browse?' + p;
  }
  function commitSearch() { const hash = browseHash(); if (location.hash === hash) search(); else location.hash = hash; }
  const detailLoads = new Map();
  async function loadDetail(v) {
    if (!v?.detail_script) return v;
    if (!globalThis.GHOSTLINE_DETAILS?.[v.id]) {
      if (!detailLoads.has(v.detail_script)) detailLoads.set(v.detail_script, new Promise((resolve, reject) => {
        const script = document.createElement('script'); script.src = v.detail_script;
        script.onload = resolve; script.onerror = () => reject(new Error('Could not load native details.'));
        document.head.append(script);
      }));
      await detailLoads.get(v.detail_script);
    }
    return globalThis.GHOSTLINE_DETAILS?.[v.id] ?? v;
  }
  async function detail(id) {
    let v = indexes.get(id);
    const root = $('detail-content'); root.replaceChildren();
    $('browse').hidden = true; $('detail').hidden = false; $('back').href = lastBrowse;
    if (!v) { root.append(el('h1', 'Entry not found'), el('p', 'This identity is not present in this catalogue.')); return; }
    root.append(el('p', 'Loading native details...', 'note'));
    try { v = await loadDetail(v); } catch (error) { root.replaceChildren(el('p', error.message, 'note')); return; }
    if (location.hash !== '#entry/' + encodeURIComponent(id)) return;
    root.replaceChildren();
    document.title = `${label(v)} \u00b7 Ghostline Items`;
    root.append(el('p', category(v), 'eyebrow'), el('h1', label(v)), el('p', id, 'native'));
    const copy = el('button', 'Copy selection reference');
    const reference = {native_name: v.native_name, source: v.source, locator: v.control ? v.id : undefined, variants: selectionIds(v).map(id => { const s = data.selections[id]; return {resource: resourceText(s.resource), appearance: s.appearance}; })};
    const referenceNode = el('pre', JSON.stringify(reference, null, 2));
    copy.onclick = async () => { try { await navigator.clipboard.writeText(referenceNode.textContent); copy.textContent = 'Copied'; } catch { const range = document.createRange(); range.selectNodeContents(referenceNode); const s = getSelection(); s.removeAllRanges(); s.addRange(range); copy.textContent = 'Reference selected'; } };
    root.append(copy, referenceNode);
    const media = previews(v);
    if (media.length) {
      const gallery = el('div', null, 'previews');
      for (const m of media) { const f = el('figure'), img = el('img'); img.src = mediaURL(m.path); img.loading = 'lazy'; img.alt = `${label(v)} \u2014 ${m.view}`; f.append(img, el('figcaption', `${m.appearance ?? 'native default'} / ${m.view} / ${m.size}px`)); gallery.append(f); } root.append(gallery);
      root.append(el('p', v.kind === 'assembly' ? 'Complete native entity in its rig rest pose, including its component placements and material bindings.' : v.kind === 'mesh' ? 'Individual native mesh. See containing assemblies for complete objects and placement context.' : 'Isolated native resource preview. Combined character fit and game material fidelity require their stated context and verification.', 'note'));
    } else root.append(el('p', 'No rendered preview is published for this entry. Native metadata and selection references remain available.', 'note'));
    for (const [field,title] of [['parts','Assembly parts'],['assemblies','Containing assemblies']]) {
      if (v[field]?.length) { root.append(el('h2', title)); const list = el('div', null, 'links'); for (const key of v[field]) list.append(link(key, label(indexes.get(key) ?? {id:key}))); root.append(list); }
    }
    const relations = [...selectionIds(v), ...(v.choices ?? []), ...(v.control ? [v.control] : [])];
    if (relations.length) { root.append(el('h2', 'Linked selections and choices')); const list = el('div', null, 'links'); for (const key of relations) list.append(link(key, label(indexes.get(key) ?? {id: key}))); root.append(list); }
    if (v.diagnostics?.length) { root.append(el('h2', 'Source and preview notes')); for (const note of v.diagnostics) root.append(el('p', note, 'note')); }
    if (v.kind === 'assembly' || v.context?.prop_components) {
      const variants = v.kind === 'assembly' ? await Promise.all(selectionIds(v).map(key => loadDetail(indexes.get(key)))) : [v];
      if (location.hash !== '#entry/' + encodeURIComponent(id)) return;
      for (const variant of variants) {
        if (!variant?.context?.prop_components?.length) continue;
        root.append(el('h2', 'Placed components / ' + (variant.appearance ?? 'native default')));
        root.append(el('p', 'Entity-relative positions in metres, native Z up. Full matrices and rig/slot bindings are in the selection JSON.', 'note'));
        const table = el('table', null, 'components'), header = el('tr');
        for (const title of ['Native component', 'Individual mesh', 'X / Y / Z']) header.append(el('th', title));
        table.append(header);
        for (const instance of variant.context.prop_components) {
          const row = el('tr'), part = el('td'); part.append(link(instance.part, label(indexes.get(instance.part) ?? {id: instance.part})));
          const position = instance.native_world_transform?.slice(12,15).map(value => value.toFixed(4)).join(' / ') ?? 'Unresolved';
          row.append(el('td', instance.component), part, el('td', position)); table.append(row);
        }
        root.append(table);
      }
    }
    root.append(el('h2', 'Native details'), el('pre', JSON.stringify(v, null, 2)));
  }
  function route() {
    if (location.hash.startsWith('#entry/')) { try { detail(decodeURIComponent(location.hash.slice(7))); } catch { detail(''); } return; }
    document.title = 'Ghostline Items'; $('browse').hidden = false; $('detail').hidden = true;
    const p = new URLSearchParams(location.hash.split('?')[1] ?? '');
    const group = p.get('collection'); $('collection').value = Object.hasOwn(groups, group) ? group : (data.items.length ? 'items' : groups.clothing.length ? 'clothing' : 'props');
    $('prop-kind').value = p.get('representation') ?? '';
    $('search').value = p.get('q') ?? ''; $('ready').checked = p.get('ready') === '1'; filterOptions(); $('category').value = p.get('category') ?? ''; if (!$('category').value) $('category').value = '';
    lastBrowse = location.hash || '#'; search();
  }
  $('search-form').onsubmit = e => { e.preventDefault(); commitSearch(); };
  $('collection').onchange = () => { $('category').value = ''; filterOptions(); commitSearch(); };
  $('category').onchange = commitSearch; $('ready').onchange = commitSearch;
  $('prop-kind').onchange = commitSearch;
  $('previous').onclick = () => { page--; draw(); $('result-count').scrollIntoView({block: 'start'}); };
  $('next').onclick = () => { page++; draw(); $('result-count').scrollIntoView({block: 'start'}); };
  addEventListener('hashchange', route); route();
})();
