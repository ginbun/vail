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
