import type { FieldRule } from '@arco-design/web-vue';

export default {
  name: [{
    required: true,
    message: '请输入标签名称'
  }, {
    maxLength: 32,
    message: '标签名称长度不能大于32位'
  }, {
    match: /^[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?$/,
    message: '标签不符合 RFC 1035 规范 (字母开头, 允许字母数字连字符, 长度1-63)'
  }],
} as Record<string, FieldRule | FieldRule[]>;
