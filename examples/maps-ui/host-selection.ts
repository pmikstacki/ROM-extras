export type SelectionOutcome = 'accepted' | 'rejected' | 'unknown';
export type SelectionRequest = Readonly<{ id: string; generation: string }>;
/** Host supplies same-origin transport and its current session generation. */
export async function requestSelection(
  id: string,
  generation: string,
  currentGeneration: () => string,
  send: (body: SelectionRequest) => Promise<unknown>,
): Promise<SelectionOutcome> {
  if (!id || id.length > 4096 || !generation || currentGeneration() !== generation) return 'rejected';
  try {
    const response = await send(Object.freeze({ id, generation }));
    if (currentGeneration() !== generation) return 'unknown';
    if (typeof response !== 'object' || response === null
      || !('generation' in response) || response.generation !== generation
      || !('outcome' in response)) return 'unknown';
    return response.outcome === 'accepted' || response.outcome === 'rejected' || response.outcome === 'unknown'
      ? response.outcome : 'unknown';
  } catch {
    // A dispatched request may have completed; lost replies never establish refusal.
    return 'unknown';
  }
}
