<template>
  <div />
</template>

<script lang="ts">
  export default {
    name: 'Redirect',
  };
</script>

<script lang="ts" setup>
  import { useRoute, useRouter } from 'vue-router';

  const route = useRoute();
  const router = useRouter();

  // 解析路径
  const [url, search] = (route.params.path as string).split('?');
  const query = Object.fromEntries(new URLSearchParams(search || ''));

  // 重定向
  router.replace({
    path: url,
    query: query as any,
  });

</script>

<style lang="less" scoped>
</style>
