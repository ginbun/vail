import assert from 'node:assert/strict';
import test from 'node:test';
import { connectWebSocketWithRetry, type WebSocketLike } from '../src/utils/websocket-connect';

class FakeSocket implements WebSocketLike {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = FakeSocket.CONNECTING;
  onopen: ((ev: Event) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  closed = false;

  open() {
    this.readyState = FakeSocket.OPEN;
    this.onopen?.(new Event('open'));
  }

  fail(reason = 'failed') {
    this.readyState = FakeSocket.CLOSED;
    this.onerror?.(new Event('error'));
    this.onclose?.({
      type: 'close',
      wasClean: false,
      code: 1006,
      reason,
    } as CloseEvent);
  }

  close() {
    this.closed = true;
    if (this.readyState === FakeSocket.CLOSED) {
      return;
    }
    this.readyState = FakeSocket.CLOSED;
    this.onclose?.({
      type: 'close',
      wasClean: true,
      code: 1000,
      reason: 'closed',
    } as CloseEvent);
  }
}

function wait(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

test('resolves when the handshake opens', async () => {
  const sockets: FakeSocket[] = [];
  const pending = connectWebSocketWithRetry('ws://example.test', {
    maxAttempts: 3,
    handshakeTimeout: 50,
  }, () => {
    const socket = new FakeSocket();
    sockets.push(socket);
    return socket;
  });

  await wait(0);
  sockets[0].open();
  const opened = await pending;
  assert.equal(opened, sockets[0]);
  assert.equal(sockets.length, 1);
});

test('retries after close and does not double-count error plus close', async () => {
  const sockets: FakeSocket[] = [];
  const pending = connectWebSocketWithRetry('ws://example.test', {
    maxAttempts: 2,
    baseDelay: 5,
    maxDelay: 5,
    jitter: false,
    handshakeTimeout: 100,
  }, () => {
    const socket = new FakeSocket();
    sockets.push(socket);
    return socket;
  });

  await wait(0);
  sockets[0].fail();
  await wait(15);
  assert.equal(sockets.length, 2);
  sockets[1].open();
  const opened = await pending;
  assert.equal(opened, sockets[1]);
});

test('handshake timeout closes the hanging socket and retries', async () => {
  const sockets: FakeSocket[] = [];
  const pending = connectWebSocketWithRetry('ws://example.test', {
    maxAttempts: 2,
    baseDelay: 5,
    maxDelay: 5,
    jitter: false,
    handshakeTimeout: 15,
  }, () => {
    const socket = new FakeSocket();
    sockets.push(socket);
    return socket;
  });

  await wait(25);
  assert.equal(sockets.length, 2);
  assert.equal(sockets[0].closed, true);
  sockets[1].open();
  await pending;
});

test('rejects after the last failed handshake attempt', async () => {
  const pending = connectWebSocketWithRetry('ws://example.test', {
    maxAttempts: 2,
    baseDelay: 5,
    maxDelay: 5,
    jitter: false,
    handshakeTimeout: 20,
  }, () => {
    const socket = new FakeSocket();
    queueMicrotask(() => socket.fail('boom'));
    return socket;
  });

  await assert.rejects(pending, (error: unknown) => {
    assert.ok(error instanceof Error || error instanceof Event);
    return true;
  });
});
