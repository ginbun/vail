import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isResumeSecurityFailure,
  isSeamlessResume,
  shouldAttemptResume,
  shouldDiscardInputOnClose,
  shouldFlushInputOnConnect,
  shouldFreshReconnectOnResumeFailure,
} from '../src/views/terminal/service/channel/ssh-resume-input-policy';

test('shouldAttemptResume requires enabled id and no force-fresh flag', () => {
  assert.equal(shouldAttemptResume(true, 'ssh-1', false), true);
  assert.equal(shouldAttemptResume(false, 'ssh-1', false), false);
  assert.equal(shouldAttemptResume(true, undefined, false), false);
  assert.equal(shouldAttemptResume(true, 'ssh-1', true), false);
});

test('isSeamlessResume only accepts exact session id match', () => {
  assert.equal(isSeamlessResume('ssh-1', 'ssh-1'), true);
  assert.equal(isSeamlessResume('ssh-2', 'ssh-1'), false);
  assert.equal(isSeamlessResume('ssh-1', undefined), false);
});

test('flush is allowed only after seamless resume connects', () => {
  assert.equal(shouldFlushInputOnConnect(true), true);
  assert.equal(shouldFlushInputOnConnect(false), false);
});

test('close discards when resume was attempted but not seamless', () => {
  assert.equal(shouldDiscardInputOnClose('ssh-1', false), true);
  assert.equal(shouldDiscardInputOnClose('ssh-1', true), false);
  assert.equal(shouldDiscardInputOnClose(undefined, false), false);
});

test('fresh connection never flushes and always discards stale buffer on open', () => {
  assert.equal(shouldAttemptResume(true, undefined, false), false);
  assert.equal(shouldFlushInputOnConnect(false), false);
  assert.equal(isSeamlessResume('ssh-new', undefined), false);
});

test('resume-not-found and buffer-gap trigger fresh reconnect fallback', () => {
  assert.equal(shouldFreshReconnectOnResumeFailure('resume-not-found'), true);
  assert.equal(shouldFreshReconnectOnResumeFailure('resume-buffer-gap'), true);
  assert.equal(shouldFreshReconnectOnResumeFailure('resume-auth-failed'), false);
  assert.equal(shouldFreshReconnectOnResumeFailure('resume-busy'), false);
  assert.equal(shouldFreshReconnectOnResumeFailure(undefined), false);
});

test('security-sensitive resume failures block fresh reconnect fallback', () => {
  assert.equal(isResumeSecurityFailure('resume-auth-failed'), true);
  assert.equal(isResumeSecurityFailure('resume-busy'), true);
  assert.equal(isResumeSecurityFailure('resume-not-found'), false);
});

test('force-fresh flag prevents resume after downgrade decision', () => {
  assert.equal(shouldAttemptResume(true, 'ssh-1', true), false);
  assert.equal(shouldFlushInputOnConnect(false), false);
});
