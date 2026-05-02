import type { ISshChannel, ISshSession } from '@/views/terminal/interfaces';
import type { OutputPayload } from '@/views/terminal/types/protocol';
import { TerminalCloseCode, TerminalMessages, TerminalSessionTypes } from '@/views/terminal/types/const';
import { ansi } from '@/utils';
import { useTerminalStore } from '@/store';
import { getTerminalAccessToken, openTerminalAccessChannel } from '@/api/terminal/terminal';
import BaseTerminalChannel from './base-terminal-channel';

// 终端通信会话 SSH 会话实现
export default class SshChannel extends BaseTerminalChannel<ISshSession> implements ISshChannel {

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
  }

  // 处理已连接消息
  processConnected(_: OutputPayload): void {
    const wasReconnecting = this.session.autoReconnectAttempts > 0;
    this.session.markAutoReconnectSucceeded?.();
    // 设置可写
    this.session.setCanWrite(true);
    // 设置已连接
    this.session.setConnected();
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
  processResize({ width, height }: OutputPayload): void {
    // this.session.resize(Number.parseInt(width), Number.parseInt(height));
  }

  // 处理 SSH 输出消息
  processSshOutput({ body }: OutputPayload): void {
    this.session.write(body);
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
      // 可以根据 meta.reason 提供更详细的错误信息
      if (meta.reason && meta.reason !== this.session.state.lastCloseReason) {
        this.session.state.lastCloseReason = meta.reason;
        // 如果需要，可以在终端显示更详细的原因
        // this.session.write('\r\n' + ansi(91, `Reason: ${meta.reason}`) + '\r\n');
      }
    } catch (e) {
      console.error('Failed to parse clmeta', e);
    }
  }

}
