import type { MapResourcePoint } from 'rom-ui/maps';

/** Host-owned authorized ROM read; IDs are passed through exactly, never geocoder IDs. */
export type AuthorizedResourceRead = (signal: AbortSignal) => Promise<readonly MapResourcePoint[]>;

/** Validate a bounded snapshot after the host performs its authorized ROM read. */
export async function loadApprovedResourcePoints(
  read: AuthorizedResourceRead,
  signal: AbortSignal,
): Promise<readonly MapResourcePoint[]> {
  signal.throwIfAborted();
  const rows = await read(signal);
  signal.throwIfAborted();
  if (rows.length > 200) throw new Error('Map point limit exceeded');
  const seen = new Set<string>();
  const approved = rows.map((row) => {
    if (typeof row.id !== 'string' || row.id.length === 0 || seen.has(row.id)) {
      throw new Error('Invalid Resource identity');
    }
    if (typeof row.title !== 'string' || row.title.length > 4096) {
      throw new Error('Invalid Resource title');
    }
    if (!Number.isFinite(row.longitude) || row.longitude < -180 || row.longitude > 180
      || !Number.isFinite(row.latitude) || row.latitude < -90 || row.latitude > 90) {
      throw new Error('Invalid Resource coordinates');
    }
    seen.add(row.id);
    return Object.freeze({ id: row.id, title: row.title, longitude: row.longitude, latitude: row.latitude });
  });
  return Object.freeze(approved);
}
