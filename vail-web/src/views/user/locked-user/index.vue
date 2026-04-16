<template>
  <div class="layout-container">
    <!-- 表格 -->
    <a-card class="general-card table-card">
      <template #title>
        <!-- 左侧操作 -->
        <div class="table-left-bar-handle">
          <!-- 标题 -->
          <div class="table-title">
            锁定用户列表
          </div>
        </div>
        <!-- 右侧操作 -->
        <div class="table-right-bar-handle">
          <a-button type="primary" @click="fetchTableData">
            查询
            <template #icon>
              <icon-search />
            </template>
          </a-button>
        </div>
      </template>
      <!-- table -->
      <a-table row-key="id"
               ref="tableRef"
               class="table-resize"
               :loading="loading"
               :columns="columns"
               :data="tableRenderData"
               :pagination="false"
               :bordered="false"
               :column-resizable="true">
        <!-- 用户名 -->
        <template #username="{ record }">
          <div style="display: contents;">
            <!-- 用户名 -->
            <span class="text-copy" @click="copy(record.username, true)">
            {{ record.username }}
          </span>
            <!-- 当前会话 -->
            <a-tag v-if="record.current"
                   size="small"
                   class="ml8"
                   color="green">
              当前
            </a-tag>
          </div>
        </template>
        <!-- 登录IP -->
        <template #address="{ record }">
          <div style="display: contents;">
          <span class="text-copy mr4" @click="copy(record.address, true)">
            {{ record.address }}
          </span>
            <span>({{ record.location }})</span>
          </div>
        </template>
        <!-- 操作 -->
        <template #handle="{ record }">
          <div class="table-handle-wrapper">
            <!-- 删除 -->
            <a-popconfirm content="确定要解锁当前用户吗?"
                          position="left"
                          type="warning"
                          @ok="unlockUser(record)">
              <a-button v-if="!record.current"
                        v-permission="['infra:system-user:management:unlock']"
                        type="text"
                        size="mini"
                        status="danger">
                解锁
              </a-button>
            </a-popconfirm>
          </div>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script lang="ts">
  export default {
    name: 'userLockedUser'
  };
</script>

<script lang="ts" setup>
  import type { LockedUserQueryResponse } from '@/api/user/user';
  import { ref, onMounted } from 'vue';
  import { Message } from '@arco-design/web-vue';
  import useLoading from '@/hooks/loading';
  import columns from './types/table.columns';
  import { copy } from '@/hooks/copy';
  import { getLockedUserList, unlockLockedUser } from '@/api/user/user';

  const { loading, setLoading } = useLoading();

  const tableRenderData = ref<Array<LockedUserQueryResponse>>([]);

  // 解锁用户
  const unlockUser = async (record: LockedUserQueryResponse) => {
    try {
      setLoading(true);
      // 解锁用户
      await unlockLockedUser({
        username: record.username,
      });
      Message.success('操作成功');
      // 查询数据
      await fetchTableData();
    } finally {
      setLoading(false);
    }
  };

  // 查询数据
  const fetchTableData = async () => {
    try {
      setLoading(true);
      const { data } = await getLockedUserList();
      tableRenderData.value = data;
    } finally {
      setLoading(false);
    }
  };

  onMounted(async () => {
    await fetchTableData();
  });

</script>

<style lang="less" scoped>

</style>
