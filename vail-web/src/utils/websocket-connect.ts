import {
  calculateRetryDelay,
  DEFAULT_RETRY_OPTIONS,
  shouldRetryHandshake,
  type WebSocketRetryOptions,
} from './websocket-policy';

export interface WebSocketLike {
  readyState: number;
  onopen: ((ev: Event) => void) | null;
  onerror: ((ev: Event) => void) | null;
  onclose: ((ev: CloseEvent) => void) | null;
  close: () => void;
}

export type WebSocketFactory<T extends WebSocketLike> = (url: string) => T;

function clearTimer(timer: ReturnType<typeof setTimeout> | undefined): undefined {
  if (timer) {
    clearTimeout(timer);
  }
  return undefined;
}

function detach(socket?: WebSocketLike): void {
  if (!socket) {
    return;
  }
  socket.onopen = null;
  socket.onerror = null;
  socket.onclose = null;
}

function closeQuietly(socket?: WebSocketLike): void {
  if (!socket) {
    return;
  }
  detach(socket);
  if (socket.readyState === 0 || socket.readyState === 1) {
    try {
      socket.close();
    } catch {
      // close can throw if the constructor failed
    }
  }
}

export function connectWebSocketWithRetry<T extends WebSocketLike>(
  url: string,
  retryOptions: WebSocketRetryOptions | undefined,
  factory: WebSocketFactory<T>,
): Promise<T> {
  const options = { ...DEFAULT_RETRY_OPTIONS, ...retryOptions };
  let attempts = 0;
  let settled = false;
  let attemptConsumed = false;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let handshakeTimer: ReturnType<typeof setTimeout> | undefined;
  let currentSocket: T | undefined;

  return new Promise<T>((resolve, reject) => {
    const finish = (error: Event | Error) => {
      if (settled) {
        return;
      }
      settled = true;
      retryTimer = clearTimer(retryTimer);
      handshakeTimer = clearTimer(handshakeTimer);
      closeQuietly(currentSocket);
      currentSocket = undefined;
      reject(error);
    };

    const retryOrFail = (error: Event | Error) => {
      if (settled || attemptConsumed) {
        return;
      }
      attemptConsumed = true;
      handshakeTimer = clearTimer(handshakeTimer);
      closeQuietly(currentSocket);
      currentSocket = undefined;

      if (shouldRetryHandshake(attempts, options.maxAttempts)) {
        retryTimer = setTimeout(connect, calculateRetryDelay(attempts, options));
        return;
      }
      finish(error);
    };

    const connect = () => {
      if (settled) {
        return;
      }
      retryTimer = clearTimer(retryTimer);
      attemptConsumed = false;
      attempts += 1;

      let socket: T;
      try {
        socket = factory(url);
      } catch (error) {
        retryOrFail(error instanceof Error ? error : new Error('websocket constructor failed'));
        return;
      }

      currentSocket = socket;
      if (options.handshakeTimeout > 0) {
        handshakeTimer = setTimeout(() => {
          retryOrFail(new Error('websocket handshake timeout'));
        }, options.handshakeTimeout);
      }

      socket.onopen = () => {
        if (settled) {
          return;
        }
        settled = true;
        handshakeTimer = clearTimer(handshakeTimer);
        retryTimer = clearTimer(retryTimer);
        detach(socket);
        resolve(socket);
      };

      socket.onerror = (event) => {
        retryOrFail(event);
      };

      socket.onclose = (event) => {
        retryOrFail(event);
      };
    };

    connect();
  });
}
