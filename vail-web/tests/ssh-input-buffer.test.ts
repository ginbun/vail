import assert from 'node:assert/strict';
import test from 'node:test';
import { SshInputBuffer } from '../src/views/terminal/service/channel/ssh-input-buffer';

test('queues and drains in original order', () => {
  const buffer = new SshInputBuffer({ maxItems: 10, maxBytes: 1024 });
  buffer.enqueue('a');
  buffer.enqueue('b');
  buffer.enqueue('c');
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, ['a', 'b', 'c']);
  assert.equal(drained.droppedCount, 0);
  assert.equal(buffer.size, 0);
});

test('drops oldest when max item count exceeded', () => {
  const buffer = new SshInputBuffer({ maxItems: 2, maxBytes: 1024 });
  buffer.enqueue('first');
  buffer.enqueue('second');
  buffer.enqueue('third');
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, ['second', 'third']);
  assert.equal(drained.droppedCount, 1);
});

test('drops oldest when max byte size exceeded', () => {
  const chunk = 'a'.repeat(600);
  const buffer = new SshInputBuffer({ maxItems: 10, maxBytes: 1024 });
  buffer.enqueue(chunk);
  buffer.enqueue(chunk);
  buffer.enqueue(chunk); // exceed -> drop oldest
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, [chunk]);
  assert.equal(drained.droppedCount, 2);
});

test('discard clears queue and returns pending metrics', () => {
  const buffer = new SshInputBuffer({ maxItems: 2, maxBytes: 4 });
  buffer.enqueue('ab');
  buffer.enqueue('cd');
  buffer.enqueue('ef');
  const discarded = buffer.discard();
  assert.deepEqual(discarded.commands, ['cd', 'ef']);
  assert.equal(discarded.droppedCount, 1);
  assert.equal(buffer.size, 0);
  assert.equal(buffer.bytes, 0);
});

test('accepts exactly maxItems without dropping', () => {
  const buffer = new SshInputBuffer({ maxItems: 2, maxBytes: 1024 });
  buffer.enqueue('a');
  buffer.enqueue('b');
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, ['a', 'b']);
  assert.equal(drained.droppedCount, 0);
});

test('accepts exactly maxBytes without dropping', () => {
  const buffer = new SshInputBuffer({ maxItems: 10, maxBytes: 1024 });
  const exact = 'a'.repeat(1024);
  buffer.enqueue(exact);
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, [exact]);
  assert.equal(drained.droppedCount, 0);
  assert.equal(buffer.bytes, 0);
});

test('drops a single item larger than maxBytes', () => {
  // maxBytes is floored to 1024 in the constructor
  const buffer = new SshInputBuffer({ maxItems: 10, maxBytes: 1024 });
  buffer.enqueue('a'.repeat(1025));
  const drained = buffer.drain();
  assert.deepEqual(drained.commands, []);
  assert.equal(drained.droppedCount, 1);
});

test('drain resets dropped count for next cycle', () => {
  const buffer = new SshInputBuffer({ maxItems: 1, maxBytes: 1024 });
  buffer.enqueue('x');
  buffer.enqueue('y'); // drop x
  const first = buffer.drain();
  assert.equal(first.droppedCount, 1);
  buffer.enqueue('z');
  const second = buffer.drain();
  assert.equal(second.droppedCount, 0);
  assert.deepEqual(second.commands, ['z']);
});

