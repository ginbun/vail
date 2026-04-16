<template>
  <div class="layout-container" v-if="render">
    <!-- 列表-表格 -->
    <tag-table ref="table"
               @open-add="(e: any) => modal.openAdd(e)"
               @open-update="(e: any) => modal.openUpdate(e)" />
    <!-- 添加修改抽屉 -->
    <tag-form-modal ref="modal"
                    @added="reload"
                    @updated="reload" />
  </div>
</template>

<script lang="ts">
  export default {
    name: 'systemTags'
  };
</script>

<script lang="ts" setup>
  import { ref, onBeforeMount } from 'vue';
  import { useDictStore } from '@/store';
  import { dictKeys } from './types/const';
  import TagTable from './components/tag-table.vue';
  import TagFormModal from './components/tag-form-modal.vue';

  const render = ref(false);
  const table = ref();
  const modal = ref();

  // 重新加载
  const reload = () => {
    table.value.reload();
  };

  onBeforeMount(async () => {
    const dictStore = useDictStore();
    await dictStore.loadKeys(dictKeys);
    render.value = true;
  });

</script>

<style lang="less" scoped>

</style>
