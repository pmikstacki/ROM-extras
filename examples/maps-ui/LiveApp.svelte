<script lang="ts">
  import ApprovedResourceMap from './ApprovedResourceMap.svelte';
  import type { MapResourcePoint } from 'rom-ui/maps';
  import { loadApprovedResourcePoints } from './host-points';
  import { requestSelection } from './host-selection';
  let points = $state<readonly MapResourcePoint[]>([]);
  let selectedId = $state<string | null>(null);
  let generation = $state('');
  let authorityToken = $state<object>({});
  let recoveryToken = $state(0);
  let status = $state('Disconnected');
  let suggestion = $state('');
  let connectionAttempt = {};
  let geocodeAttempt = {};
  async function post(path: string, body?: unknown): Promise<unknown> {
    const response = await fetch(path, { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
    if (!response.ok) throw new Error('Host request unavailable');
    return response.status === 204 ? null : response.json();
  }
  async function connect() {
    const attempt = {};
    connectionAttempt = attempt;
    try {
    points = []; selectedId = null; suggestion = ''; generation = ''; authorityToken = {};
    const session = await post('/fixture/session');
    if (connectionAttempt !== attempt) return;
    if (typeof session !== 'object' || session === null || !('generation' in session) || typeof session.generation !== 'string') throw new Error('Invalid session');
    generation = session.generation;
    const expected = generation;
    const response = await fetch('/points', { credentials: 'same-origin', cache: 'no-store' });
    if (connectionAttempt !== attempt) return;
    if (!response.ok) throw new Error('Host read unavailable');
    const snapshot: unknown = await response.json();
    if (connectionAttempt !== attempt) return;
    if (typeof snapshot !== 'object' || snapshot === null || !('generation' in snapshot) || snapshot.generation !== expected || !('points' in snapshot) || !Array.isArray(snapshot.points)) throw new Error('Invalid snapshot');
    const rows = snapshot.points.map((row: unknown) => {
      if (typeof row !== 'object' || row === null || !('id' in row) || typeof row.id !== 'string' || !('title' in row) || typeof row.title !== 'string' || !('longitude' in row) || typeof row.longitude !== 'number' || !('latitude' in row) || typeof row.latitude !== 'number') throw new Error('Invalid point');
      return { id: row.id, title: row.title, longitude: row.longitude, latitude: row.latitude };
    });
    const approved = await loadApprovedResourcePoints(async () => rows, new AbortController().signal);
    if (generation !== expected) return;
    points = approved; status = 'Connected'; recoveryToken += 1;
    } catch {
      if (connectionAttempt !== attempt) return;
      generation = ''; points = []; selectedId = null; authorityToken = {}; status = 'Host unavailable';
    }
  }
  async function select(id: string) {
    const expected = generation;
    const outcome = await requestSelection(id, expected, () => generation, body => post('/selection', body));
    if (outcome === 'accepted' && generation === expected) selectedId = id;
    return outcome;
  }
  async function revoke() {
    const attempt = {};
    connectionAttempt = attempt;
    generation = ''; points = []; selectedId = null; suggestion = ''; authorityToken = {}; status = 'Local access cleared; revocation pending';
    try {
      await post('/fixture/revoke');
      if (connectionAttempt === attempt) status = 'Revoked';
    } catch {
      if (connectionAttempt === attempt) status = 'Local access cleared; host revocation unknown';
    }
  }
  async function geocode() {
    const expected = generation;
    if (!expected) return;
    const authority = connectionAttempt;
    const attempt = {};
    geocodeAttempt = attempt;
    const current = () => generation === expected && connectionAttempt === authority && geocodeAttempt === attempt;
    suggestion = '';
    try {
    const response = await post('/geocode', { generation: expected, text: 'Warsaw & test', limit: 1 });
    if (!current()) return;
    if (typeof response !== 'object' || response === null || !('generation' in response) || response.generation !== expected || !('suggestions' in response) || !Array.isArray(response.suggestions)) throw new Error('Invalid suggestion');
    if (response.suggestions.length === 0) { suggestion = 'No results'; return; }
    const result: unknown = response.suggestions[0];
    if (typeof result !== 'object' || result === null || !('label' in result) || typeof result.label !== 'string' || !('attribution' in result) || typeof result.attribution !== 'string') throw new Error('Invalid suggestion');
    suggestion = `${result.label}; Credit: ${result.attribution}`;
    } catch {
      if (current()) suggestion = 'Geocoding unavailable';
    }
  }
  async function simulateUnknown() {
    const authority = connectionAttempt;
    try {
      await post('/fixture/unknown');
    } catch {
      if (connectionAttempt === authority) status = 'Host control outcome unknown';
    }
  }
</script>
<h1>Live controlled ROM host</h1>
<p>Local fixture authentication and blank map style.</p>
<button onclick={connect}>Connect host fixture</button>
<button onclick={revoke}>Revoke host fixture</button>
<button onclick={geocode} disabled={!generation}>Query host geocoder</button>
<button onclick={simulateUnknown} disabled={!generation}>Simulate unknown host outcome</button>
<p data-testid="host-status">{status}</p>
<p data-testid="selection">Selected: {selectedId ?? 'none'}</p>
<p data-testid="live-suggestion">{suggestion}</p>
<div class="map-frame"><ApprovedResourceMap {points} {selectedId} {authorityToken} {recoveryToken} onSelect={select} /></div>
<style>
  :global(body) { margin: 1rem; font-family: sans-serif; }
  .map-frame { height: 480px; position: relative; }
  :global(.h-full) { height: 100%; }
  :global(.w-full) { width: 100%; }
  :global(.relative) { position: relative; }
</style>
