import type { App, ComponentPublicInstance } from 'vue';

/**
 * 全局异常处理
 */
export default function globalErrorHandler(Vue: App) {
  Vue.config.errorHandler = (
    err: unknown,
    instance: ComponentPublicInstance | null,
    info: string
  ) => {
    console.error(info, err);
  };
}
