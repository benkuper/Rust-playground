<script lang="ts">
  import PanelShell from "../../components/PanelShell.svelte";
  import ParamControl from "../../components/ParamControl.svelte";
  import { engineStore } from "../../stores/engine";

  const { selectedNode, nodeById, paramById } = engineStore;

  const collectParamNodes = (rootId: number | string, map: Map<any, any>) => {
    const result: Array<number | string> = [];
    const stack: Array<number | string> = [rootId];
    while (stack.length) {
      const nodeId = stack.pop();
      const node = map.get(nodeId);
      if (!node) continue;
      if (node.data?.kind === "Parameter") {
        result.push(nodeId);
      }
      if (node.children?.length) {
        for (const child of node.children) {
          stack.push(child);
        }
      }
    }
    return result;
  };
</script>

<PanelShell title="Parameters" subtitle="Live edit the selected subtree">
  {#if $selectedNode}
    {#each collectParamNodes($selectedNode.node_id, $nodeById) as paramNodeId (paramNodeId)}
      {#if $paramById.get(paramNodeId)}
        <ParamControl
          param={$paramById.get(paramNodeId)}
          onChange={(value) => engineStore.setParam(paramNodeId, value)}
        />
      {/if}
    {/each}
  {:else}
    <p class="panel-subtitle">Select a node to edit its parameters.</p>
  {/if}
</PanelShell>
