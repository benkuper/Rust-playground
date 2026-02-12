<script lang="ts">
  import PanelShell from "../../components/PanelShell.svelte";
  import { engineStore } from "../../stores/engine";

  const { events } = engineStore;

  const eventLabel = (event: { kind?: Record<string, unknown> | null }) =>
    event.kind ? Object.keys(event.kind)[0] : "Event";
</script>

<PanelShell title="Events" subtitle="Recent engine events">
  {#if $events.length}
    <div class="list">
      {#each $events as event (event.time.seq)}
        <div class="event-row">
          <div class="node-row">
            <span class="node-kind">{eventLabel(event)}</span>
            <span class="node-label">node {event.node}</span>
            <span class="node-meta">
              <span class="tag">t={event.time.tick}.{event.time.micro}</span>
            </span>
          </div>
          <div class="mono">{JSON.stringify(event.kind)}</div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="panel-subtitle">No events yet.</p>
  {/if}
</PanelShell>
