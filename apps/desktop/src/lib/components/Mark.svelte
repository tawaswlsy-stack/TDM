<script lang="ts">
  import mark from "../../../../../assets/icon/TDM_Glyph_Cyan.svg?raw";
  let { size = 26 }: { size?: number } = $props();
</script>

<span class="mark" style:width="{size}px" style:height="{size}px" aria-hidden="true">{@html mark}</span>

<style>
  .mark {
    display: inline-flex;
    color: var(--accent);
    flex: none;
  }
  .mark :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
