import { webSocketBaseUrl } from '@/utils/env';

/**
 * WebSocket 重试配置
 */
export interface WebSocketRetryOptions {
  maxAttempts?: number;
  baseDelay?: number;
  maxDelay?: number;
  jitter?: boolean;
}

const DEFAULT_RETRY_OPTIONS: WebSocketRetryOptions = {
  maxAttempts: 0, // 默认不重试
  baseDelay: 1000,
  maxDelay: 30000,
  jitter: true,
};

/**
 * 创建应用 websocket
 */
export const createAppWebSocket = (url: string, retryOptions?: WebSocketRetryOptions): Promise<WebSocket> => {
  return createWebSocket(webSocketBaseUrl + url, retryOptions);
};

/**
 * 创建 websocket
 */
export const createWebSocket = (url: string, retryOptions?: WebSocketRetryOptions): Promise<WebSocket> => {
  const options = { ...DEFAULT_RETRY_OPTIONS, ...retryOptions };
  let attempts = 0;
  let settled = false;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let retryScheduled = false;

  return new Promise<WebSocket>((resolve, reject) => {
    const connect = () => {
      retryScheduled = false;
      attempts++;
      const socket = new WebSocket(url);

      socket.onopen = () => {
        if (retryTimer) {
          clearTimeout(retryTimer);
          retryTimer = undefined;
        }
        settled = true;
        resolve(socket);
      };

      socket.onerror = (e) => {
        if (settled || retryScheduled) {
          return;
        }
        if (options.maxAttempts && attempts < options.maxAttempts) {
          const delay = calculateDelay(attempts, options);
          retryScheduled = true;
          retryTimer = setTimeout(connect, delay);
        } else {
          settled = true;
          reject(e);
        }
      };
    };

    connect();
  });
};

/**
 * 计算重试延迟
 */
function calculateDelay(attempt: number, options: WebSocketRetryOptions): number {
  const baseDelay = options.baseDelay || 1000;
  const maxDelay = options.maxDelay || 30000;
  let delay = Math.min(maxDelay, baseDelay * Math.pow(2, attempt - 1));

  if (options.jitter) {
    delay = delay * (0.5 + Math.random() * 0.5);
  }

  return delay;
}
