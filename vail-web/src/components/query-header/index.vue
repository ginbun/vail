<template>
  <div class="query-header">
    <a-space wrap>
      <slot />
    </a-space>
  </div>
</template>

<script lang="ts">
  export default {
    name: 'QueryHeader'
  };
</script>

<style lang="less" scoped>
  .query-header {
    margin-bottom: 16px;
    background: var(--color-bg-2);
    padding: 16px;
    border-radius: 4px;
  }
</style>
