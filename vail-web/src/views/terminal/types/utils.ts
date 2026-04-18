import { Message } from '@arco-design/web-vue';

// 获取显示大小
export const getDisplaySize = (size: string, tips: boolean = false): [number, number] => {
  if (size?.includes('x')) {
    const [w, h] = size.split('x');
    return [Number.parseInt(w), Number.parseInt(h)];
  }
  if (tips) {
    Message.error('分辨率格式不正确, 请重新选择或输入 (如: 800x600)');
  }
  throw new Error('Invalid size');
};

// 截屏 (已禁用: 安全风险)
export const screenshot = async (el: HTMLElement) => {
  console.log('screenshot disabled', el);
  Message.warning('截屏功能暂未开放');
};
