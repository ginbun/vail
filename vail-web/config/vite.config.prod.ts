import { mergeConfig } from 'vite';
import baseConfig from './vite.config.base';
import configVisualizerPlugin from './plugin/visualizer';
import configArcoResolverPlugin from './plugin/arcoResolver';

export default mergeConfig(
  {
    mode: 'production',
    plugins: [
      configVisualizerPlugin(),
      configArcoResolverPlugin(),
    ],
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes('node_modules')) {
              if (id.includes('@arco-design/web-vue')) return 'arco';
              if (id.includes('echarts') || id.includes('vue-echarts')) return 'chart';
              if (id.includes('vue') || id.includes('vue-router') || id.includes('pinia') || id.includes('@vueuse/core') || id.includes('vue-i18n')) return 'vue';
              if (id.includes('axios')) return 'axios';
              if (id.includes('@xterm')) return 'xterm';
              if (id.includes('monaco-editor')) return 'monaco';
              if (id.includes('dayjs') || id.includes('cron-parser')) return 'pkg';
              return 'vendor';
            }
          },
        },
      },
      chunkSizeWarningLimit: 1024 * 8,
    },
  },
  baseConfig,
);
