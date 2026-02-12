<script lang="ts">
    import TreeView from "./TreeView.svelte";
    import { formatValue } from "../utils/values";

    type NodeId = number | string;
    type NodeMeta = { label: string; short_name: string };
    type NodeDto = {
        node_id: NodeId;
        node_type: string;
        meta: NodeMeta;
        children?: NodeId[];
    };
    type ParamDto = { param_node_id: NodeId; value: unknown };

    type Props = {
        root?: NodeDto | null;
        nodeById?: Map<NodeId, NodeDto>;
        paramById?: Map<NodeId, ParamDto>;
        selectedId?: NodeId | null;
        onSelect?: (nodeId: NodeId) => void;
    };

    const {
        root = null,
        nodeById = new Map(),
        paramById = new Map(),
        selectedId = null,
        onSelect = () => {},
    } = $props<Props>();

    const isSelected = (nodeId: NodeId) => selectedId === nodeId;
</script>

{#if root}
    <div class="list">
        <button
            type="button"
            class="node-row"
            class:selected={isSelected(root.node_id)}
            onclick={() => onSelect(root.node_id)}
        >
            <span class="node-kind">{root.node_type}</span>
            <!-- <span class="node-label">{root.meta.label}</span> -->
            <span class="node-meta">
                <span class="badge">{root.meta.short_name}</span>
                <!-- <span class="mono"
                    >{formatValue(paramById?.get(root.node_id)?.value)}</span
                > -->
            </span>
        </button>
        {#if root.children && root.children.length}
            <div class="list">
                {#each root.children as childId (childId)}
                    {#if nodeById?.get(childId)}
                        <TreeView
                            root={nodeById.get(childId)}
                            {nodeById}
                            {paramById}
                            {selectedId}
                            {onSelect}
                        />
                    {/if}
                {/each}
            </div>
        {/if}
    </div>
{:else}
    <p class="panel-subtitle">No root node yet.</p>
{/if}

<style>
    .list {
        margin-left:.5rem;
    }
    .selected {
        border-color: var(--accent);
        box-shadow: 0 10px 20px var(--glow);
    }
</style>
