/** Whether this connection attempt should carry resume auth fields. */
export function shouldAttemptResume(
  resumeEnabled: boolean | undefined,
  resumeSessionId: string | undefined,
  forceFreshSession: boolean,
): boolean {
  return !!(resumeEnabled && resumeSessionId && !forceFreshSession);
}

/** Server `id|` matches the session we tried to resume. */
export function isSeamlessResume(
  sessionId: string,
  resumeAttemptSessionId: string | undefined,
): boolean {
  return !!resumeAttemptSessionId && sessionId === resumeAttemptSessionId;
}

/** Buffered keystrokes may be replayed only after seamless resume connects. */
export function shouldFlushInputOnConnect(resumeSeamlessConnected: boolean): boolean {
  return resumeSeamlessConnected;
}

/** Failed or downgraded resume attempts must not replay buffered input later. */
export function shouldDiscardInputOnClose(
  resumeAttemptSessionId: string | undefined,
  resumeSeamlessConnected: boolean,
): boolean {
  return !!resumeAttemptSessionId && !resumeSeamlessConnected;
}

/** Resume failures that should downgrade to a fresh SSH session. */
export function shouldFreshReconnectOnResumeFailure(reason: string | undefined): boolean {
  return reason === 'resume-not-found' || reason === 'resume-buffer-gap';
}

/** Security-sensitive resume failures must not auto-fallback to a fresh session. */
export function isResumeSecurityFailure(reason: string | undefined): boolean {
  return reason === 'resume-auth-failed' || reason === 'resume-busy';
}

/** First resume-capable retry stays silent; later or non-resume retries are visible. */
export function shouldAnnounceAutoReconnect(
  scheduled: boolean,
  autoReconnectAttempts: number,
  canAttemptResume: boolean,
): boolean {
  if (!scheduled) {
    return false;
  }
  return !canAttemptResume || autoReconnectAttempts > 1;
}

/** Seamless resume should not print a recovery banner. */
export function shouldAnnounceReconnectSuccess(
  wasReconnecting: boolean,
  seamlessResume: boolean,
): boolean {
  return wasReconnecting && !seamlessResume;
}

/** Do not ask the operator to press Enter while auto-reconnect is already running. */
export function shouldPromptManualReconnect(
  canReconnect: boolean,
  autoReconnectScheduled: boolean,
): boolean {
  return canReconnect && !autoReconnectScheduled;
}

/** Hide the disconnect banner only for a mid-session, resume-capable auto-reconnect. */
export function shouldWriteDisconnectNotice(
  beforeConnected: boolean,
  autoReconnectScheduled: boolean,
  canAttemptResume: boolean,
): boolean {
  return !(beforeConnected && autoReconnectScheduled && canAttemptResume);
}
