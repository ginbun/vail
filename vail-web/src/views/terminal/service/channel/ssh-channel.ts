import type { ISshChannel, ISshSession } from '@/views/terminal/interfaces';
import type { InputPayload, OutputPayload, Protocol } from '@/views/terminal/types/protocol';
import { InputProtocol } from '@/views/terminal/types/protocol';
import { TerminalCloseCode, TerminalMessages, TerminalSessionTypes } from '@/views/terminal/types/const';
import { ansi } from '@/utils';
import { useTerminalStore } from '@/store';
import { getTerminalAccessToken, openTerminalAccessChannel } from '@/api/terminal/terminal';
import BaseTerminalChannel from './base-terminal-channel';
import { SshInputBuffer } from './ssh-input-buffer';
import {
  isSeamlessResume,
  shouldAttemptResume,
  shouldDiscardInputOnClose,
  shouldFlushInputOnConnect,
} from './ssh-resume-input-policy';

// 终端通信会话 SSH 会话实现
export default class SshChannel extends BaseTerminalChannel<ISshSession> implements ISshChannel {
  private readonly pendingInput = new SshInputBuffer();
  private isFlushingInput = false;
  private resumeAttemptSessionId?: string;
  private resumeSeamlessConnected = false;

  // 打开 channel
  protected async openChannel(): Promise<void> {
    const { preference } = useTerminalStore();
    const { data } = await getTerminalAccessToken({
      hostId: this.session.info.hostId,
      connectType: TerminalSessionTypes.SSH.type,
      extra: {
        terminalType: preference.sshInteractSetting.terminalEmulationType ?? 'xterm',
      }
    });
    // 打开 channel
    this.client = await openTerminalAccessChannel(TerminalSessionTypes.SSH.channel, data, {
      maxAttempts: 3,
      baseDelay: 1000,
      jitter: true,
    });
    const shouldResume = shouldAttemptResume(
      data.resume?.enabled,
      this.session.resumeSessionId,
      this.session.forceFreshSession,
    );
    this.resumeAttemptSessionId = shouldResume ? this.session.resumeSessionId : undefined;
    this.resumeSeamlessConnected = false;
    if (!shouldResume) {
      this.discardPendingInput(TerminalMessages.reconnectInputDiscarded);
    }
    const authPayload: Record<string, unknown> = {
      type: 'auth',
      ticket: data.wsTicket,
      sessionHint: data.sessionHint,
    };
    if (shouldResume) {
      authPayload.resumeSessionId = this.session.resumeSessionId;
      authPayload.resumeLastOffset = this.session.lastOutputOffset;
    }
    this.client.send(JSON.stringify(authPayload));
  }

  processSetId({ sessionId }: OutputPayload): void {
    const seamlessResume = isSeamlessResume(sessionId, this.resumeAttemptSessionId);
    this.resumeSeamlessConnected = seamlessResume;
    if (!seamlessResume) {
      this.discardPendingInput(TerminalMessages.reconnectInputDiscarded);
    }
    super.processSetId({ type: 'id', sessionId } as OutputPayload);
    this.session.resumeSessionId = sessionId;
    if (this.session.forceFreshSession) {
      this.session.forceFreshSession = false;
      this.session.lastOutputOffset = 0;
    }
  }

  // 处理已连接消息
  processConnected(payload: OutputPayload): void {
    void payload;
    const wasReconnecting = this.session.autoReconnectAttempts > 0;
    this.session.markAutoReconnectSucceeded?.();
    // 设置可写
    this.session.setCanWrite(true);
    // 设置已连接
    this.session.setConnected();
    if (shouldFlushInputOnConnect(this.resumeSeamlessConnected)) {
      this.flushPendingInput();
    } else {
      this.discardPendingInput(TerminalMessages.reconnectInputDiscarded);
    }
    if (wasReconnecting) {
      this.session.write(ansi(92, `\r\n${TerminalMessages.reconnectSuccess}\r\n`));
    }
  }

  // 处理已已关闭消息
  processClosed({ code, msg }: OutputPayload): void {
    if (this.triggerClosed) {
      return;
    }
    const beforeConnected = this.session.state.connected;
    this.triggerClosed = true;
    // 设置重连状态
    const codeNumber = Number.parseInt(code);
    this.session.state.canReconnect = TerminalCloseCode.FORCE !== codeNumber;
    if (shouldDiscardInputOnClose(this.resumeAttemptSessionId, this.resumeSeamlessConnected)) {
      this.discardPendingInput(TerminalMessages.reconnectInputDiscarded);
    }
    // 拼接关闭消息
    this.session.write((beforeConnected ? '\r\n\r\n' : '') + ansi(91, msg || ''));
    if (codeNumber === TerminalCloseCode.NETWORK) {
      const scheduled = this.session.scheduleAutoReconnect?.();
      if (scheduled) {
        this.session.write('\r\n' + ansi(91, TerminalMessages.autoReconnecting) + '\r\n');
      }
    }
    if (this.session.state.canReconnect) {
      this.session.write('\r\n' + ansi(91, TerminalMessages.waitingReconnect) + '\r\n');
    }
    // 设置已关闭
    this.session.setClosed();
    // 关闭 channel
    this.close();
  }

  // 处理修改大小
  processResize(payload: OutputPayload): void {
    void payload;
    // this.session.resize(Number.parseInt(width), Number.parseInt(height));
  }

  // 处理 SSH 输出消息
  processSshOutput({ body }: OutputPayload): void {
    this.session.write(body);
    this.session.lastOutputOffset += new TextEncoder().encode(body || '').length;
  }

  send(protocol: Protocol, payload?: InputPayload): void {
    if (protocol.type !== InputProtocol.SSH_INPUT.type) {
      super.send(protocol, payload);
      return;
    }
    const command = String(payload?.command ?? '');
    if (this.isOpened() && !this.isFlushingInput) {
      super.send(protocol, payload);
      return;
    }
    this.pendingInput.enqueue(command);
  }

  // 处理关闭元数据
  processClMeta({ body }: OutputPayload): void {
    try {
      const meta = JSON.parse(body);
      // 如果 meta 标记为可重试，且尚未触发自动重连，则尝试触发
      if (meta.retryable && !this.session.autoReconnectTimer) {
        const scheduled = this.session.scheduleAutoReconnect?.();
        if (scheduled) {
          this.session.write('\r\n' + ansi(91, TerminalMessages.autoReconnecting) + '\r\n');
        }
      }
      if (meta.reason === 'resume-buffer-gap') {
        this.discardPendingInput(TerminalMessages.reconnectInputDiscarded);
        this.session.forceFreshSession = true;
        this.session.resumeSessionId = undefined;
        this.session.lastOutputOffset = 0;
        this.session.state.canReconnect = true;
        setTimeout(() => {
          useTerminalStore().reOpenSession(this.session.sessionKey);
        }, 0);
      }
      // 可以根据 meta.reason 提供更详细的错误信息
      if (meta.reason && meta.reason !== this.session.state.lastCloseReason) {
        this.session.state.lastCloseReason = meta.reason;
        // 如果需要，可以在终端显示更详细的原因
        // this.session.write('\r\n' + ansi(91, `Reason: ${meta.reason}`) + '\r\n');
      }
    } catch (error) {
      console.error('Failed to parse clmeta', error);
    }
  }

  private flushPendingInput(): void {
    if (this.isFlushingInput || !this.isOpened()) {
      return;
    }
    this.isFlushingInput = true;
    try {
      while (this.pendingInput.size > 0) {
        const { commands, droppedCount } = this.pendingInput.drain();
        for (const command of commands) {
          super.send(InputProtocol.SSH_INPUT, { command });
        }
        if (droppedCount > 0) {
          this.session.write(ansi(93, `\r\n${TerminalMessages.reconnectInputDropped}\r\n`));
        }
      }
    } finally {
      this.isFlushingInput = false;
    }
  }

  private discardPendingInput(message: string): void {
    const { commands, droppedCount } = this.pendingInput.discard();
    if (commands.length === 0 && droppedCount === 0) {
      return;
    }
    this.session.write(ansi(93, `\r\n${message}\r\n`));
    if (droppedCount > 0) {
      this.session.write(ansi(93, `\r\n${TerminalMessages.reconnectInputDropped}\r\n`));
    }
  }

}
