import type { TerminalTheme } from '@/views/terminal/interfaces';
import axios from 'axios';
import { createAppWebSocket, type WebSocketRetryOptions } from '@/utils/http';

// 终端访问请求
export interface TerminalAccessRequest {
  hostId?: number;
  connectType?: string;
  extra?: Record<string, any>;
}

export interface TerminalAccessResumeConfig {
  enabled: boolean;
  windowSeconds: number;
}

export interface TerminalAccessV2Response {
  accessId: string;
  wsUrl: string;
  wsTicket: string;
  expiresAt: number;
  sessionHint: string;
  resume: TerminalAccessResumeConfig;
}

/**
 * 获取主机终端主题
 */
export function getTerminalThemes() {
  return axios.get<Array<TerminalTheme>>('/terminal/terminal/themes');
}

/**
 * 获取主机终端 accessToken
 */
export function getTerminalAccessToken(request: TerminalAccessRequest) {
  return axios.post<TerminalAccessV2Response>('/terminal/terminal/access', request);
}

/**
 * 获取主机终端 transferToken
 */
export function getTerminalTransferToken() {
  return axios.get<string>('/terminal/terminal/transfer');
}

/**
 * 打开主机终端 websocket
 */
export const openTerminalAccessChannel = (
  protocol: string,
  access: TerminalAccessV2Response,
  retryOptions?: WebSocketRetryOptions
) => {
  if (!access.wsUrl) {
    return createAppWebSocket(`/terminal/access/${protocol}`, retryOptions);
  }
  return createAppWebSocket(access.wsUrl, retryOptions);
};

/**
 * 打开主机传输 websocket
 */
export const openTerminalTransferChannel = (accessToken: string, retryOptions?: WebSocketRetryOptions) => {
  return createAppWebSocket(`/terminal/transfer/${accessToken}`, retryOptions);
};
