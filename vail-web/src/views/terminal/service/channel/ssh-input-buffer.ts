export interface SshInputBufferOptions {
  maxItems?: number;
  maxBytes?: number;
}

export interface SshInputDrainResult {
  commands: string[];
  droppedCount: number;
}

const encoder = new TextEncoder();

interface BufferedCommand {
  value: string;
  bytes: number;
}

// SSH 输入有界缓冲（断连窗口用）
export class SshInputBuffer {
  private readonly maxItems: number;
  private readonly maxBytes: number;
  private totalBytes = 0;
  private droppedCount = 0;
  private readonly queue: BufferedCommand[] = [];

  constructor(options: SshInputBufferOptions = {}) {
    this.maxItems = Math.max(1, options.maxItems ?? 2048);
    this.maxBytes = Math.max(1024, options.maxBytes ?? 64 * 1024);
  }

  get size(): number {
    return this.queue.length;
  }

  get bytes(): number {
    return this.totalBytes;
  }

  enqueue(command: string): void {
    const value = command ?? '';
    const bytes = encoder.encode(value).length;
    const item: BufferedCommand = { value, bytes };
    this.queue.push(item);
    this.totalBytes += bytes;
    this.trimToBounds();
  }

  // 取出当前缓冲并清空（保持 FIFO 顺序）
  drain(): SshInputDrainResult {
    const commands = this.queue.map(item => item.value);
    this.queue.length = 0;
    this.totalBytes = 0;
    const droppedCount = this.droppedCount;
    this.droppedCount = 0;
    return { commands, droppedCount };
  }

  // 丢弃所有缓冲，返回丢弃统计用于提示
  discard(): SshInputDrainResult {
    return this.drain();
  }

  private trimToBounds(): void {
    while (this.queue.length > this.maxItems || this.totalBytes > this.maxBytes) {
      const removed = this.queue.shift();
      if (!removed) {
        break;
      }
      this.totalBytes -= removed.bytes;
      this.droppedCount += 1;
    }
  }
}

