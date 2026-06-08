import type { ITerminalSession, IGuacdChannel, GuacdReactiveSessionStatus } from '@/views/terminal/interfaces';
import type { OutputPayload } from '../../types/protocol';
import { InputProtocol, OutputProtocol } from '../../types/protocol';
import { TerminalCloseCode, TerminalMessages } from '@/views/terminal/types/const';
import Guacamole from 'guacamole-common-js';
import BaseTerminalChannel from './base-terminal-channel';

export const PING_FREQUENCY = 500;

// 自适应心跳上下限（毫秒）
const MIN_PING_FREQUENCY = 500;
const MAX_PING_FREQUENCY = 5000;
const MIN_UNSTABLE_THRESHOLD = 3000;
const MAX_UNSTABLE_THRESHOLD = 15000;
const MIN_RECEIVE_TIMEOUT = 30000;
const MAX_RECEIVE_TIMEOUT = 60000;

// 上下限裁剪
const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

// guacd 通信处理器基类 实现
export default abstract class BaseGuacdChannel<T extends ITerminalSession<GuacdReactiveSessionStatus>>
  extends BaseTerminalChannel<T>
  implements Guacamole.Tunnel, IGuacdChannel {

  public state: Guacamole.Tunnel.State;
  public uuid: string | null;
  public receiveTimeout: number;
  public unstableThreshold: number;
  public receiveTimeoutId?: number;
  public unstableTimeoutId?: number;
  public pingTimeoutId?: number;
  public lastSentTime: number;

  // 自适应心跳：当前 ping 频率、平滑 RTT、在途 ping 时间戳
  public pingFrequency: number;
  private smoothedRtt: number;
  private pingInFlightAt: number;

  public onuuid: ((uuid: string) => void) | null;
  public onerror: ((status: Guacamole.Status) => void) | null;
  public oninstruction: ((opcode: string, args: unknown[]) => void) | null;
  public onstatechange: ((state: Guacamole.Tunnel.State) => void) | null;

  constructor(session: T) {
    super(session);
    this.uuid = null;
    this.lastSentTime = 0;
    this.receiveTimeout = MIN_RECEIVE_TIMEOUT;
    this.unstableThreshold = MIN_UNSTABLE_THRESHOLD;
    this.pingFrequency = PING_FREQUENCY;
    this.smoothedRtt = 0;
    this.pingInFlightAt = 0;
    this.state = Guacamole.Tunnel.State.CLOSED;
    this.onuuid = null;
    this.onerror = null;
    this.oninstruction = null;
    this.onstatechange = null;
  }

  // 打开 channel
  protected abstract openChannel(): Promise<void>;

  // 连接会话 guacd 内部调用
  async connect(connectData: string): Promise<void> {
    void connectData;
    // 重置计时器
    this.resetTimers();
    // 未开启则初始化
    if (!this.isOpened()) {
      // 设置状态
      this.setState(Guacamole.Tunnel.State.CONNECTING);
      // 初始化
      await this.init();
    }
    // 重置计时器
    this.resetTimers();
  }

  // 处理设置id
  processSetId(payload: OutputPayload) {
    // 设置父类 sessionId
    super.processSetId(payload);
    // 设置内部 uuid
    const { sessionId } = payload;
    this.uuid = sessionId;
    this.onuuid?.(sessionId);
    // 设置状态为开启
    this.setState(Guacamole.Tunnel.State.OPEN);
  }

  // 处理指令
  processInstruction({ instruction }: OutputPayload): void {
    // 收到任意数据视为一次往返样本，更新自适应心跳
    this.recordRttSample();
    // 重置计时器
    this.resetTimers();
    let startIndex = 0;
    let elementEnd = undefined as unknown as number;
    let elements = [];
    do {
      // 搜索结束长度
      const lengthEnd = instruction.indexOf('.', startIndex);
      if (lengthEnd !== -1) {
        // 消息长度
        const length = parseInt(instruction.substring(elementEnd + 1, lengthEnd));
        // 计算开始索引
        startIndex = lengthEnd + 1;
        // 计算结束索引
        elementEnd = startIndex + length;
      } else {
        // 错误指令
        this.closeTunnel(Guacamole.Status.Code.SERVER_ERROR, 'Incomplete instruction.');
      }
      // 获取元素
      const element = instruction.substring(startIndex, elementEnd);
      const terminator = instruction.substring(elementEnd, elementEnd + 1);
      elements.push(element);
      if (terminator === ';') {
        // 操作码
        const opcode = elements.shift();
        // 指令回调
        if (opcode !== Guacamole.Tunnel.INTERNAL_DATA_OPCODE && this.oninstruction) {
          this.oninstruction(opcode as string, elements);
        }
        // 清除数据
        elements.length = 0;
      }
      // 继续处理指令
      startIndex = elementEnd + 1;
    } while (startIndex < instruction.length);
  }

  // 处理已连接消息 需要在状态切换时手动调用
  processConnected(payload: OutputPayload): void {
    void payload;
    // 设置可写
    this.session.setCanWrite(true);
    // 设置已连接
    this.session.setConnected();
  }

  // 处理已关闭消息
  processClosed({ code, msg }: OutputPayload) {
    if (this.triggerClosed) {
      return;
    }
    this.triggerClosed = true;
    // 关闭计时器
    this.clearTimers();
    // 通知错误
    const closeCode = Number.parseInt(code) || TerminalCloseCode.NORMAL;
    if (closeCode !== Guacamole.Status.Code.SUCCESS && this.onerror) {
      this.onerror(new Guacamole.Status(closeCode as any, msg));
    }
    // 设置关闭原因
    this.session.state.closeCode = closeCode;
    this.session.state.closeMessage = msg || TerminalMessages.sessionClosed;
    // 设置重连状态
    this.session.state.canReconnect = TerminalCloseCode.FORCE !== closeCode;
    // 设置已关闭
    this.setState(Guacamole.Tunnel.State.CLOSED);
    this.session.setClosed();
    // 关闭会话
    this.close();
  }

  // 处理 pong 消息：用于测量 RTT
  processPong(): void {
    this.recordRttSample();
  }

  // 记录一次 RTT 样本并据此自适应调整心跳参数
  private recordRttSample(): void {
    if (this.pingInFlightAt <= 0) {
      return;
    }
    const sample = Date.now() - this.pingInFlightAt;
    this.pingInFlightAt = 0;
    if (sample < 0) {
      return;
    }
    // 指数加权平均，降低抖动影响
    this.smoothedRtt = this.smoothedRtt > 0
      ? Math.round(this.smoothedRtt * 0.7 + sample * 0.3)
      : sample;
    // 高延迟弱网下降低 ping 频率，避免心跳风暴；提高不稳定阈值，避免误判
    this.pingFrequency = clamp(Math.round(this.smoothedRtt * 1.5), MIN_PING_FREQUENCY, MAX_PING_FREQUENCY);
    this.unstableThreshold = clamp(Math.round(this.smoothedRtt * 4), MIN_UNSTABLE_THRESHOLD, MAX_UNSTABLE_THRESHOLD);
    this.receiveTimeout = clamp(Math.round(this.smoothedRtt * 10), MIN_RECEIVE_TIMEOUT, MAX_RECEIVE_TIMEOUT);
  }

  // 发送 ping
  ping() {
    if (Date.now() < this.lastSentTime + this.pingFrequency) {
      return;
    }
    // 记录在途 ping 时间用于 RTT 测量（仅在没有未完成样本时）
    if (this.pingInFlightAt <= 0) {
      this.pingInFlightAt = Date.now();
    }
    // this.sendInstruction(Guacamole.Tunnel.INTERNAL_DATA_OPCODE, 'ping', Date.now());
    this.send(InputProtocol.PING);
  }

  // 发送指令 guacd 内部调用
  sendMessage(...messages: any[]): void {
    this.sendInstruction(...messages);
  }

  // 发送指令
  sendInstruction(...messages: any[]): void {
    // 连接状态检查
    if (!this.isConnected()) {
      return;
    }
    // 防御性检查
    const length = messages.length;
    if (length === 0) {
      return;
    }
    // 添加初始消息
    let instruction = '';
    // 添加后续元素
    for (let i = 0; i < length; i++) {
      if (i > 0) {
        instruction += ',';
      }
      instruction += this.getElement(messages[i]);
    }
    // 尾部间隔符
    instruction += ';';
    // 发送消息
    this.send(InputProtocol.GUACD_INSTRUCTION, { instruction });
    // 设置最后发送时间
    this.lastSentTime = Date.now();
  }

  // 断开连接
  disconnect() {
    // 手动关闭 内部调用
    this.closeTunnel(Guacamole.Status.Code.SUCCESS, TerminalMessages.sessionClosed);
  };

  // 关闭 tunnel
  closeTunnel(code: number, msg?: string): void {
    this.processClosed({
      type: OutputProtocol.CLOSED.type,
      code: String(code || TerminalCloseCode.NORMAL),
      msg: msg || TerminalMessages.sessionClosed,
    });
  }

  // 重置计时器
  private resetTimers(): void {
    // 关闭计时器
    this.clearTimers();
    // 重置状态
    if (this.state === Guacamole.Tunnel.State.UNSTABLE) {
      this.setState(Guacamole.Tunnel.State.OPEN);
    }
    // 重置超时计时器
    this.receiveTimeoutId = window.setTimeout(() => {
      this.closeTunnel(Guacamole.Status.Code.UPSTREAM_TIMEOUT, 'Server timeout.');
    }, this.receiveTimeout);
    // 重置不稳定计时器
    this.unstableTimeoutId = window.setTimeout(() => {
      this.setState(Guacamole.Tunnel.State.UNSTABLE);
    }, this.unstableThreshold);
    // 检查发送 ping（使用自适应频率）
    if (Date.now() < this.lastSentTime + this.pingFrequency) {
      this.pingTimeoutId = window.setTimeout(this.ping.bind(this), this.pingFrequency);
    } else {
      this.ping();
    }
  }

  // 关闭计时器
  private clearTimers() {
    clearTimeout(this.receiveTimeoutId);
    clearTimeout(this.unstableTimeoutId);
    clearTimeout(this.pingTimeoutId);
  }

  // 获取元素
  private getElement(value: any): string {
    const string = String(value);
    return string.length + '.' + string;
  }

  // 设置状态
  private setState(state: Guacamole.Tunnel.State) {
    // 通知状态变化
    if (state !== this.state) {
      this.state = state;
      this.onstatechange?.(state);
    }
  }

  // guacd 是否已连接
  isConnected() {
    return this.state === Guacamole.Tunnel.State.OPEN || this.state === Guacamole.Tunnel.State.UNSTABLE;
  };

}

