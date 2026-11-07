<script lang="ts">
  // التخطيط العام
</script>

<div class="app-wrapper">
  <slot />
</div>

<style>
  :global(html),
  :global(body) {
    background-color: #e3e3db !important;
    color: #6c6d6c;
    margin: 0;
    padding: 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    min-height: 100vh;
    direction: rtl;
  }

  .app-wrapper {
    min-height: 100vh;
    background-color: #e3e3db;
    padding-bottom: 2rem;
  }
</style>
