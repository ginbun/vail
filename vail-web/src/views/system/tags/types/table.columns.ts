import type { TableColumnData } from '@arco-design/web-vue';
import { dateFormat } from '@/utils';

const columns = [
  {
    title: 'id',
    dataIndex: 'id',
    slotName: 'id',
    width: 68,
    align: 'left',
    fixed: 'left',
    default: true,
  }, {
    title: '标签名称',
    dataIndex: 'name',
    slotName: 'name',
    align: 'left',
    minWidth: 168,
    ellipsis: true,
    tooltip: true,
    default: true,
  }, {
    title: '关联数量',
    dataIndex: 'relCount',
    slotName: 'relCount',
    align: 'left',
    minWidth: 168,
    ellipsis: true,
    tooltip: true,
    default: true,
  }, {
    title: '创建时间',
    dataIndex: 'createTime',
    slotName: 'createTime',
    align: 'center',
    width: 180,
    render: ({ record }) => {
      return dateFormat(new Date(record.createTime));
    },
    default: true,
  }, {
    title: '修改时间',
    dataIndex: 'updateTime',
    slotName: 'updateTime',
    align: 'center',
    width: 180,
    render: ({ record }) => {
      return dateFormat(new Date(record.updateTime));
    },
  }, {
    title: '创建人',
    width: 148,
    dataIndex: 'creator',
    slotName: 'creator',
    default: true,
  }, {
    title: '修改人',
    width: 148,
    dataIndex: 'updater',
    slotName: 'updater',
  }, {
    title: '操作',
    slotName: 'handle',
    width: 138,
    align: 'center',
    fixed: 'right',
    default: true,
  },
] as TableColumnData[];

export default columns;
