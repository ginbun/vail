<template>
  <a-select
:model-value="modelValue"
            :placeholder="placeholder"
            :options="optionData"
            :loading="loading"
            :limit="limit as number"
            :allow-create="allowCreate"
            :separator="','"
            :filter-option="labelFilter"
            multiple
            allow-clear
            @change="onChange"
            @exceed-limit="() => { emits('onLimited', limit, `最多选择${limit}个tag`) }">
    <!-- 选项 -->
    <template #option="{ data: { label } }">
      <a-tag :color="dataColor(label, tagColor)">
        {{ label }}
      </a-tag>
    </template>
  </a-select>
</template>

<script lang="ts">
  export default {
    name: 'TagMultiSelector'
  };
</script>

<script lang="ts" setup>
  import type { SelectOptionData } from '@arco-design/web-vue';
  import type { TagCreateRequest } from '@/api/meta/tag';
  import { ref, computed, onMounted, onActivated } from 'vue';
  import { useCacheStore } from '@/store';
  import { dataColor } from '@/utils';
  import { labelFilter } from '@/types/form';
  import { createTag } from '@/api/meta/tag';
  import useLoading from '@/hooks/loading';

  const props = withDefaults(defineProps<Partial<{
    modelValue: Array<number>;
    placeholder: string;
    limit: number;
    type: string;
    allowCreate: boolean;
    tagColor: Array<string>;
  }>>(), {
    allowCreate: false,
    tagColor: () => [],
  });

  const emits = defineEmits(['update:modelValue', 'onLimited', 'onIllegal']);

  const { loading, setLoading } = useLoading();
  const cacheStore = useCacheStore();

  const RFC_1035_REGEXP = /^[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?$/;

  const optionData = ref<SelectOptionData[]>([]);

  // 处理变化
  const onChange = async (e: any) => {
    const values = await checkCreateTag(e as Array<any>);
    emits('update:modelValue', values);
  };

  // 检查并创建 tag，返回处理后的 id 数组
  const checkCreateTag = async (tags: Array<any>): Promise<number[]> => {
    if (!tags || !tags.length) {
      return [];
    }
    const result: number[] = [];
    for (const tag of tags) {
      // 如果是数字，说明是已存在的 tag ID
      if (typeof tag === 'number') {
        result.push(tag);
        continue;
      }
      
      // 如果是字符串，说明是新输入的标签
      // 1. 验证是否已在选项中存在（可能用户输入了已存在的标签名但没从下拉选）
      const existingOption = optionData.value.find((o) => o.label === tag);
      if (existingOption) {
        result.push(existingOption.value as number);
        continue;
      }

      // 2. 验证 RFC 1035
      if (!RFC_1035_REGEXP.test(tag)) {
        emits('onIllegal', `标签 ${tag} 不符合 RFC 1035 规范 (字母开头, 允许字母数字连字符, 长度1-63)`);
        continue;
      }

      // 3. 不存在则创建 tag
      setLoading(true);
      try {
        const id = await doCreateTag(tag);
        result.push(id);
      } catch (e) {
        // 忽略创建失败的
      } finally {
        setLoading(false);
      }
    }
    return result;
  };

  // 创建 tag
  const doCreateTag = async (name: string) => {
    const { data: id } = await createTag({
      name,
      type: props.type
    } as unknown as TagCreateRequest);
    // 插入缓存
    const tagCache = await cacheStore.loadTags(props.type as string);
    tagCache && tagCache.push({ id, name });
    // 插入 options
    optionData.value.push({
      label: name,
      value: id,
      tagProps: {
        color: dataColor(name, props.tagColor)
      }
    });
    return id;
  };

  // 初始化选项
  const initOptions = async () => {
    setLoading(true);
    try {
      const tags = await cacheStore.loadTags(props.type as string);
      optionData.value = tags.map(s => {
        return {
          label: s.name,
          value: s.id,
          tagProps: {
            color: dataColor(s.name, props.tagColor)
          }
        };
      });
    } finally {
      setLoading(false);
    }
  };

  // 初始化选项
  onMounted(initOptions);
  onActivated(initOptions);

</script>

<style lang="less" scoped>

</style>
