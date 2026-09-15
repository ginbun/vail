export interface WebSocketRetryOptions {
  maxAttempts?: number;
  baseDelay?: number;
  maxDelay?: number;
  jitter?: boolean;
  handshakeTimeout?: number;
}

export const DEFAULT_HANDSHAKE_TIMEOUT_MS = 10_000;
export const DEFAULT_PING_INTERVAL_MS = 15_000;
export const DEFAULT_PING_STALE_AFTER_MS = DEFAULT_PING_INTERVAL_MS * 2;
export const PONG_WATCHDOG_MS = 8_000;

export const DEFAULT_RETRY_OPTIONS: Required<WebSocketRetryOptions> = {
  maxAttempts: 0,
  baseDelay: 1000,
  maxDelay: 30_000,
  jitter: true,
  handshakeTimeout: DEFAULT_HANDSHAKE_TIMEOUT_MS,
};

export const WEAK_NETWORK_HANDSHAKE_RETRY: Required<WebSocketRetryOptions> = {
  maxAttempts: 3,
  baseDelay: 1000,
  maxDelay: 30_000,
  jitter: true,
  handshakeTimeout: DEFAULT_HANDSHAKE_TIMEOUT_MS,
};

export function calculateRetryDelay(
  attempt: number,
  options: Pick<WebSocketRetryOptions, 'baseDelay' | 'maxDelay' | 'jitter'>,
): number {
  const baseDelay = options.baseDelay || 1000;
  const maxDelay = options.maxDelay || 30_000;
  let delay = Math.min(maxDelay, baseDelay * Math.pow(2, attempt - 1));

  if (options.jitter) {
    delay = delay * (0.5 + Math.random() * 0.5);
  }

  return delay;
}

export function shouldRetryHandshake(attempts: number, maxAttempts: number | undefined): boolean {
  return !!maxAttempts && attempts < maxAttempts;
}

export function isWebSocketStale(
  lastInboundAt: number,
  now: number,
  staleAfterMs = DEFAULT_PING_STALE_AFTER_MS,
): boolean {
  if (lastInboundAt <= 0) {
    return false;
  }
  return now - lastInboundAt >= staleAfterMs;
}

export function shouldDeclareMissedPong(
  pingInFlightAt: number,
  now: number,
  watchdogMs = PONG_WATCHDOG_MS,
): boolean {
  return pingInFlightAt > 0 && now - pingInFlightAt >= watchdogMs;
}
