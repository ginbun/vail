import type { FieldRule } from '@arco-design/web-vue';

export default {
  name: [{
    required: true,
    message: '请输入标签名称'
  }, {
    maxLength: 32,
    message: '标签名称长度不能大于32位'
  }],
} as Record<string, FieldRule | FieldRule[]>;
