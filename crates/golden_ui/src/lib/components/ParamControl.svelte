<script lang="ts">
  import { buildValue, constraintBounds, formatValue, unwrapValue } from "../utils/values";

  type ParamDto = {
    value: unknown;
    constraints?: unknown;
    semantics?: { intent?: string | null } | null;
  };

  type NodeDto = 
  {
    meta?: { label?: String | null } | null
  }

  type Props = {
    node?: NodeDto | null
    param?: ParamDto | null;
    onChange?: (value: unknown) => void;
  };

  const { node = null, param = null, onChange = () => {} } = $props<Props>();

  const info = $derived(unwrapValue(param?.value));
  const bounds = $derived(constraintBounds(param?.constraints));

  const emit = (value: unknown) => {
    onChange(buildValue(info.kind, value));
  };

  const onNumberInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement;
    const raw = target.value;
    const value = raw === "" ? 0 : Number(raw);
    emit(info.kind === "Int" ? Math.trunc(value) : value);
  };

</script>

{#if param}
  <div class="param-control">
    <div>
      <div class="param-label">{param.semantics?.intent ?? node.meta.label}</div>
      <div class="mono">{formatValue(param.value)}</div>
    </div>

    {#if info.kind === "Bool"}
      <label class="toggle">
        <input
          type="checkbox"
          checked={info.value}
          onchange={(event) => emit((event.currentTarget as HTMLInputElement).checked)}
        />
        <span>{info.value ? "On" : "Off"}</span>
      </label>
    {:else if info.kind === "String"}
      <input
        type="text"
        value={info.value}
        onchange={(event) => emit((event.currentTarget as HTMLInputElement).value)}
        placeholder="value"
      />
    {:else if info.kind === "Trigger"}
      <button onclick={() => emit(null)} class="nav-item">Fire</button>
    {:else if info.kind === "Enum"}
      <div class="panel-subtitle">Enum editing not wired yet.</div>
    {:else if info.kind === "Reference"}
      <div class="panel-subtitle">Reference: {info.value?.uuid}</div>
    {:else}
      <div class="list">
        <input
          type="range"
          min={bounds.min ?? 0}
          max={bounds.max ?? 1}
          step={bounds.step ?? 0.01}
          value={info.value}
          oninput={onNumberInput}
        />
        <input
          type="number"
          min={bounds.min}
          max={bounds.max}
          step={bounds.step ?? 0.01}
          value={info.value}
          onchange={onNumberInput}
        />
      </div>
    {/if}
  </div>
{/if}
