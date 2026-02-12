<script lang="ts">
  import { onMount } from "svelte";
  import { engineStore } from "../lib/stores/engine";
  import { panels } from "../lib/panels";
  import { appConfig } from "../lib/app/config";

  const { status, eventTime } = engineStore;

  let activePanels = $state(
    panels
      .filter((panel) => panel.defaultOpen)
      .map((panel) => panel.id)
  );

  const togglePanel = (panelId: string) => {
    if (activePanels.includes(panelId)) {
      activePanels = activePanels.filter((id) => id !== panelId);
    } else {
      activePanels = [...activePanels, panelId];
    }
  };

  const isActive = (panelId: string) => activePanels.includes(panelId);

  onMount(() => {
    engineStore.connect();
  });
</script>

<div class="app-shell">
  <aside class="side-nav">
    <div class="brand">
      <h1>{appConfig.title}</h1>
      <p>{appConfig.subtitle}</p>
    </div>

    <div class="nav-section">
      <span class="nav-title">Core Panels</span>
      {#each panels as panel (panel.id)}
        <button class="nav-item" onclick={() => togglePanel(panel.id)}>
          <span>{panel.title}</span>
          <span class="badge">{isActive(panel.id) ? "On" : "Off"}</span>
        </button>
      {/each}
    </div>

    <div class="nav-section">
      <span class="nav-title">Status</span>
      <div class="nav-item">
        <span>Connection</span>
        <span class="badge">{$status.state}</span>
      </div>
      <div class="nav-item">
        <span>Tick</span>
        <span class="badge">{$eventTime.tick}</span>
      </div>
    </div>
  </aside>

  <main class="content">
    <header class="topbar">
      <div>
        <h2>Golden Core UI</h2>
        <p class="panel-subtitle">{appConfig.statusNote}</p>
      </div>
      <div class="status-pill">
        <span class="badge">{$status.state}</span>
        <span class="mono">{$status.detail}</span>
      </div>
    </header>

    <section class="panel-grid">
      {#each panels as panel (panel.id)}
        {#if isActive(panel.id)}
          {@const Panel = panel.component}
          <Panel />
        {/if}
      {/each}
    </section>
  </main>
</div>
