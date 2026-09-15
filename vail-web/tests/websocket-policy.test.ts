import assert from 'node:assert/strict';
import test from 'node:test';
import {
  calculateRetryDelay,
  isWebSocketStale,
  shouldDeclareMissedPong,
  shouldRetryHandshake,
  DEFAULT_HANDSHAKE_TIMEOUT_MS,
  DEFAULT_PING_STALE_AFTER_MS,
  PONG_WATCHDOG_MS,
  WEAK_NETWORK_HANDSHAKE_RETRY,
} from '../src/utils/websocket-policy';

test('retry delay uses exponential backoff capped by maxDelay', () => {
  const options = { baseDelay: 1000, maxDelay: 8000, jitter: false };
  assert.equal(calculateRetryDelay(1, options), 1000);
  assert.equal(calculateRetryDelay(2, options), 2000);
  assert.equal(calculateRetryDelay(3, options), 4000);
  assert.equal(calculateRetryDelay(4, options), 8000);
  assert.equal(calculateRetryDelay(5, options), 8000);
});

test('retry delay jitter stays within half to full backoff window', () => {
  const options = { baseDelay: 1000, maxDelay: 30000, jitter: true };
  for (let i = 0; i < 20; i += 1) {
    const delay = calculateRetryDelay(3, options);
    assert.ok(delay >= 2000);
    assert.ok(delay <= 4000);
  }
});

test('handshake retries until maxAttempts inclusive of the first try', () => {
  assert.equal(shouldRetryHandshake(1, 3), true);
  assert.equal(shouldRetryHandshake(2, 3), true);
  assert.equal(shouldRetryHandshake(3, 3), false);
  assert.equal(shouldRetryHandshake(1, 0), false);
});

test('stale detection waits for two missed keepalive windows', () => {
  assert.equal(isWebSocketStale(0, 60_000), false);
  assert.equal(isWebSocketStale(10_000, 39_999), false);
  assert.equal(isWebSocketStale(10_000, 40_000), true);
  assert.equal(isWebSocketStale(10_000, 40_000, 15_000), true);
});

test('weak-network handshake defaults cover timeout and retry', () => {
  assert.equal(WEAK_NETWORK_HANDSHAKE_RETRY.maxAttempts, 3);
  assert.equal(WEAK_NETWORK_HANDSHAKE_RETRY.handshakeTimeout, DEFAULT_HANDSHAKE_TIMEOUT_MS);
  assert.equal(DEFAULT_PING_STALE_AFTER_MS, 30_000);
  assert.equal(PONG_WATCHDOG_MS, 8_000);
});

test('missed pong watchdog fires only after an in-flight ping expires', () => {
  assert.equal(shouldDeclareMissedPong(0, 20_000), false);
  assert.equal(shouldDeclareMissedPong(10_000, 17_999), false);
  assert.equal(shouldDeclareMissedPong(10_000, 18_000), true);
});
