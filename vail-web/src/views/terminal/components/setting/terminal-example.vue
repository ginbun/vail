<template>
  <div ref="viewport" class="terminal-example" />
</template>

<script lang="ts">
  export default {
    name: 'TerminalExample'
  };
</script>

<script lang="ts" setup>
  import type { TerminalThemeSchema } from '@/views/terminal/interfaces';
  import { Terminal } from '@xterm/xterm';
  import { markRaw, onMounted, onUnmounted, ref } from 'vue';

  const props = defineProps<{
    schema: TerminalThemeSchema | Record<string, any>;
  }>();

  const viewport = ref();
  const term = ref();

  onMounted(() => {
    const terminal = new Terminal({
      theme: { ...props.schema, cursor: props.schema.background },
      cols: 42,
      rows: 6,
      fontSize: 15,
      cursorInactiveStyle: 'none',
    });
    terminal.open(viewport.value);
    terminal.write(
      '\x1B[1;94m[root \x1B[0m@\x1B[1;96mOrionServer usr]# \x1B[0m\r\n' +
      'dr-xr-xr-x.  2 root root  \x1B[0m\x1B[01;34mbin \x1B[0m\r\n' +
      'dr-xr-xr-x.  2 root root  \x1B[01;34msbin \x1B[0m\r\n' +
      'drwxr-xr-x.  4 root root  \x1B[01;34msrc \x1B[0m\r\n' +
      'lrwxrwxrwx.  1 root root  \x1B[01;36mtmp \x1B[0m -> \x1B[30;42m../var/tmp \x1B[0m '
    );
    term.value = markRaw(terminal);
  });

  defineExpose({ term });

  onUnmounted(() => {
    term.value?.dispose();
  });

</script>

<style lang="less" scoped>
  .terminal-example {
    padding: 16px;
    width: 100%;
    height: 100%;
  }

  :deep(.xterm-viewport) {
    overflow: hidden !important;
  }
</style>
