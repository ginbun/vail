<template>
  <div class="icon-picker">
    <slot name="iconSelect">
      <a-input v-model="icon" placeholder="请输入图标名称">
        <template #prefix>
          <component :is="icon" v-if="icon" />
        </template>
      </a-input>
    </slot>
  </div>
</template>

<script lang="ts">
  export default {
    name: 'IconPicker'
  };
</script>

<script lang="ts" setup>
  import { computed } from 'vue';

  const props = defineProps<{
    icon?: string;
  }>();

  const emit = defineEmits(['update:icon']);

  const icon = computed({
    get: () => props.icon,
    set: (val) => emit('update:icon', val)
  });
</script>
