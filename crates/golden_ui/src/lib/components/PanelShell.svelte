<script lang="ts">
  type Props = {
    title?: string;
    subtitle?: string;
    children: () => unknown;
  };

  const { title = "", subtitle = "", children } = $props<Props>();
</script>

<section class="panel fade-in">
  <header>
    <h2>{title}</h2>
    {#if subtitle}
      <p class="panel-subtitle">{subtitle}</p>
    {/if}
  </header>
  {@render children()}
</section>
