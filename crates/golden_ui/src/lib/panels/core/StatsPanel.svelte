<script lang="ts">
  import PanelShell from "../../components/PanelShell.svelte";
  import { engineStore } from "../../stores/engine";

  const { nodes, params, eventTime, status } = engineStore;
</script>

<PanelShell title="Engine Pulse" subtitle="Snapshot counters">
  <div class="list">
    <div class="node-row">
      <span class="node-kind">Nodes</span>
      <span class="node-label">{$nodes.length}</span>
      <span class="node-meta"><span class="tag">total</span></span>
    </div>
    <div class="node-row">
      <span class="node-kind">Params</span>
      <span class="node-label">{$params.length}</span>
      <span class="node-meta"><span class="tag">live</span></span>
    </div>
    <div class="node-row">
      <span class="node-kind">Tick</span>
      <span class="node-label">{$eventTime.tick}</span>
      <span class="node-meta"><span class="tag">{$eventTime.micro}</span></span>
    </div>
    <div class="node-row">
      <span class="node-kind">Status</span>
      <span class="node-label">{$status.state}</span>
      <span class="node-meta"><span class="tag">{$status.detail}</span></span>
    </div>
  </div>
</PanelShell>
