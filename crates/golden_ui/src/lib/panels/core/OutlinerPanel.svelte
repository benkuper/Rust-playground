<script lang="ts">
  import PanelShell from "../../components/PanelShell.svelte";
  import TreeView from "../../components/TreeView.svelte";
  import { engineStore } from "../../stores/engine";

  type NodeDto = { meta?: { decl_id?: string } };

  let search = $state("");

  const { nodes, nodeById, paramById, selection } = engineStore;

  const rootNode = ($nodes: NodeDto[]) => $nodes.find((node) => node.meta?.decl_id === "root");

  const onSelect = (nodeId: number | string) => {
    engineStore.setSelection(nodeId);
  };
</script>

<PanelShell title="Outliner" subtitle="Live hierarchy + fast selection">
  <input
    type="text"
    placeholder="Filter by label"
    value={search}
    oninput={(event) => (search = (event.currentTarget as HTMLInputElement).value)}
  />

  {#if $nodes?.length}
    {#if rootNode($nodes)}
      <TreeView
        root={rootNode($nodes)}
        nodeById={$nodeById}
        paramById={$paramById}
        selectedId={$selection.nodeId}
        {onSelect}
      />
    {:else}
      <p class="panel-subtitle">Root node not found.</p>
    {/if}
  {:else}
    <p class="panel-subtitle">Waiting for snapshot...</p>
  {/if}
</PanelShell>
