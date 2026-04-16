<template>
  <!-- 内容部分 -->
  <div class="container-content">
    <!-- 业务类型 -->
    <a-card class="general-card table-search-card biz-card">
      <a-tabs v-model:active-key="tagType"
              direction="vertical"
              type="rounded"
              :hide-content="true"
              @change="reload">
        <a-tab-pane v-for="item in toOptions(TagTypeKey)"
                    :key="item.value as string"
                    :title="item.label" />
      </a-tabs>
    </a-card>
    <!-- 表格 -->
    <a-card class="general-card table-card">
      <template #title>
        <!-- 左侧操作 -->
        <div class="table-left-bar-handle">
          <!-- 标题 -->
          <div class="table-title">
            标签列表
          </div>
        </div>
        <!-- 右侧操作 -->
        <div class="table-right-bar-handle">
          <a-space>
            <!-- 标签名称 -->
            <a-input v-model="formModel.name"
                     style="width: 168px;"
                     placeholder="标签名称"
                     allow-clear
                     @press-enter="reload" />
            <!-- 重置 -->
            <a-button v-permission="['infra:tag:create']"
                      @click="resetQuery">
              重置
              <template #icon>
                <icon-refresh />
              </template>
            </a-button>
            <!-- 查询 -->
            <a-button v-permission="['infra:tag:create']"
                      type="primary"
                      @click="() => fetchTableData()">
              查询
              <template #icon>
                <icon-search />
              </template>
            </a-button>
            <a-divider direction="vertical"
                       style="height: 22px; margin: 0 8px"
                       :size="2" />
            <!-- 新增 -->
            <a-button v-permission="['infra:tag:create']"
                      type="primary"
                      @click="emits('openAdd', tagType)">
              <template #icon>
                <icon-plus />
              </template>
              新增
            </a-button>
            <!-- 调整 -->
            <table-adjust :columns="columns"
                          :columns-hook="columnsHook"
                          :query-order="queryOrder"
                          @query="fetchTableData" />
          </a-space>
        </div>
      </template>
      <!-- table -->
      <a-table row-key="id"
               ref="tableRef"
               class="table-resize"
               :loading="loading"
               :columns="tableColumns"
               :data="tableRenderData"
               :pagination="pagination"
               :bordered="false"
               :column-resizable="true"
               @page-change="(page: number) => fetchTableData(page, pagination.pageSize)"
               @page-size-change="(size: number) => fetchTableData(1, size)">
        <!-- 标签名称 -->
        <template #name="{ record }">
          <a-tag :color="dataColor(record.name, TagColors)">
            {{ record.name }}
          </a-tag>
        </template>
        <!-- 关联数量 -->
        <template #relCount="{ record }">
          <span class="span-blue">
            {{ record.relCount }} 个
          </span>
        </template>
        <!-- 操作 -->
        <template #handle="{ record }">
          <div class="table-handle-wrapper">
            <!-- 修改 -->
            <a-button v-permission="['infra:tag:update']"
                      type="text"
                      size="mini"
                      @click="emits('openUpdate', record)">
              修改
            </a-button>
            <!-- 删除 -->
            <a-popconfirm content="确认删除这个标签以及关联关系吗?"
                          position="left"
                          type="warning"
                          @ok="deleteRow(record)">
              <a-button v-permission="['infra:tag:delete']"
                        type="text"
                        size="mini"
                        status="danger">
                删除
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
    name: 'systemTagTable'
  };
</script>

<script lang="ts" setup>
  import type { TagQueryRequest, TagQueryResponse } from '@/api/meta/tag';
  import { reactive, ref, onMounted } from 'vue';
  import { deleteTag, getTagPage } from '@/api/meta/tag';
  import { Message } from '@arco-design/web-vue';
  import useLoading from '@/hooks/loading';
  import columns from '../types/table.columns';
  import { TableName, TagTypeKey } from '../types/const';
  import { useTablePagination, useTableColumns } from '@/hooks/table';
  import { useDictStore } from '@/store';
  import { useQueryOrder, ASC } from '@/hooks/query-order';
  import { dataColor } from '@/utils';
  import { TagColors } from '@/types/const';
  import TableAdjust from '@/components/app/table-adjust/index.vue';

  const emits = defineEmits(['openAdd', 'openUpdate']);

  const pagination = useTablePagination();
  const { loading, setLoading } = useLoading();
  const queryOrder = useQueryOrder(TableName, ASC);
  const { tableColumns, columnsHook } = useTableColumns(TableName, columns);
  const { toOptions } = useDictStore();

  const tagType = ref('HOST');

  const tableRenderData = ref<Array<TagQueryResponse>>([]);
  const formModel = reactive<TagQueryRequest>({
    name: undefined,
  });

  // 删除当前行
  const deleteRow = async (record: TagQueryResponse) => {
    try {
      setLoading(true);
      // 调用删除接口
      await deleteTag(record.id);
      Message.success('删除成功');
      // 重新加载
      reload();
    } finally {
      setLoading(false);
    }
  };

  // 重新加载
  const reload = () => {
    // 重新加载数据
    fetchTableData();
  };

  defineExpose({ reload });

  // 重置查询
  const resetQuery = () => {
    formModel.name = undefined;
    fetchTableData();
  };

  // 加载数据
  const doFetchTableData = async (request: TagQueryRequest) => {
    try {
      setLoading(true);
      const { data } = await getTagPage(queryOrder.markOrderly({ ...request, type: tagType.value }));
      tableRenderData.value = data.rows;
      pagination.total = data.total;
      pagination.current = request.page;
      pagination.pageSize = request.limit;
    } finally {
      setLoading(false);
    }
  };

  // 切换页码
  const fetchTableData = (page = 1, limit = pagination.pageSize, form = formModel) => {
    doFetchTableData({ page, limit, ...form });
  };

  onMounted(() => {
    fetchTableData();
  });

</script>

<style lang="less" scoped>
  @biz-card-width: 148px;
  .container-content {
    display: flex;
  }

  .biz-card {
    width: @biz-card-width;
    margin: 0 16px 0 0 !important;
    user-select: none;
  }

  .table-card {
    width: calc(100% - @biz-card-width - 16px);
  }
</style>
