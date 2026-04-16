import type { TableColumnData } from '@arco-design/web-vue';
import { dateFormat } from '@/utils';

const columns = [
  {
    title: '序号',
    dataIndex: 'seq',
    slotName: 'seq',
    width: 100,
    align: 'left',
    fixed: 'left',
    default: true,
    render: ({ rowIndex }) => {
      return rowIndex + 1;
    },
  }, {
    title: '用户名',
    dataIndex: 'username',
    slotName: 'username',
    width: 188,
    ellipsis: true,
    tooltip: true,
    default: true,
  }, {
    title: '登录IP',
    dataIndex: 'address',
    slotName: 'address',
    minWidth: 228,
    default: true,
  }, {
    title: 'userAgent',
    dataIndex: 'userAgent',
    slotName: 'userAgent',
    minWidth: 328,
    ellipsis: true,
    tooltip: true,
    default: true,
  }, {
    title: '登录时间',
    dataIndex: 'loginTime',
    slotName: 'loginTime',
    align: 'center',
    width: 198,
    render: ({ record }) => {
      return dateFormat(new Date(record.loginTime));
    },
    default: true,
  }, {
    title: '操作',
    slotName: 'handle',
    width: 108,
    align: 'center',
    fixed: 'right',
    default: true,
  },
] as TableColumnData[];

export default columns;
