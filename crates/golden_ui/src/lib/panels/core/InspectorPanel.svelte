<script lang="ts">
  import PanelShell from "../../components/PanelShell.svelte";
  import { engineStore } from "../../stores/engine";
  import { formatValue } from "../../utils/values";

  const { selectedNode, paramById } = engineStore;
</script>

<PanelShell title="Inspector" subtitle="Selected node details">
  {#if $selectedNode}
    <div class="list">
      <div class="node-row">
        <span class="node-kind">Type</span>
        <span class="node-label">{$selectedNode.node_type}</span>
        <span class="node-meta"><span class="tag">{$selectedNode.meta.short_name}</span></span>
      </div>
      <div class="node-row">
        <span class="node-kind">Label</span>
        <span class="node-label">{$selectedNode.meta.label}</span>
        <span class="node-meta"><span class="tag">{$selectedNode.meta.enabled ? "Enabled" : "Disabled"}</span></span>
      </div>
      <div class="node-row">
        <span class="node-kind">Decl</span>
        <span class="node-label">{$selectedNode.meta.decl_id}</span>
        <span class="node-meta"><span class="mono">{$selectedNode.node_id}</span></span>
      </div>
      <div class="node-row">
        <span class="node-kind">Value</span>
        <span class="node-label">{formatValue($paramById.get($selectedNode.node_id)?.value)}</span>
        <span class="node-meta"><span class="tag">{$selectedNode.meta.semantics?.intent ?? ""}</span></span>
      </div>
    </div>
  {:else}
    <p class="panel-subtitle">Select a node to inspect.</p>
  {/if}
</PanelShell>
