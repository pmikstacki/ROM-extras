<script lang="ts">
  import { ResourceMap, type MapResourcePoint } from 'rom-ui/maps';
  import type { ComponentProps } from 'svelte';
  type SelectResult = Awaited<ReturnType<ComponentProps<typeof ResourceMap>['onSelect']>>;
  let {
    points,
    selectedId,
    authorityToken,
    recoveryToken,
    onSelect,
  }: {
    points: readonly MapResourcePoint[];
    selectedId: string | null;
    authorityToken: unknown;
    recoveryToken: unknown;
    onSelect: (id: string) => SelectResult | Promise<SelectResult>;
  } = $props();
</script>

<ResourceMap
  {points}
  {selectedId}
  {authorityToken}
  {recoveryToken}
  {onSelect}
  label="Authorized Resources"
  failedLabel="The host rejected this selection."
  unknownLabel="The selection outcome is unknown. Ask the host to recover."
/>
