import { webSocketBaseUrl } from '@/utils/env';
import { connectWebSocketWithRetry } from '@/utils/websocket-connect';
import type { WebSocketRetryOptions } from '@/utils/websocket-policy';

export type { WebSocketRetryOptions } from '@/utils/websocket-policy';
export { WEAK_NETWORK_HANDSHAKE_RETRY } from '@/utils/websocket-policy';

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
  return connectWebSocketWithRetry(url, retryOptions, (target) => new WebSocket(target));
};
