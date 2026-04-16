<template>
  <a-modal v-model:visible="visible"
           top="80px"
           title-align="start"
           title="文件上传"
           ok-text="上传"
           :body-style="{ padding: 0 }"
           :align-center="false"
           :mask-closable="false"
           :unmount-on-close="true"
           :on-before-ok="handleOk"
           @cancel="handleClose">
    <a-form :model="formModel"
            ref="formRef"
            label-align="right"
            :auto-label-width="true">
      <!-- 上传目录 -->
      <a-form-item field="parentPath"
                   label="上传目录"
                   :rules="[{ required: true, message: '请输入文件上传目录' }]">
        <a-textarea ref="pathRef"
                    v-model="formModel.parentPath"
                    placeholder="上传目录"
                    :auto-size="{ minRows: 3, maxRows: 3 }" />
      </a-form-item>
      <!-- 选择文件 -->
      <a-form-item class="mb0" hide-asterisk>
        <div class="button-container">
          <!-- 选择文件 -->
          <a-upload v-model:file-list="fileList"
                    :auto-upload="false"
                    :show-file-list="false"
                    :multiple="true">
            <template #upload-button>
              <a-button type="primary" long>选择文件</a-button>
            </template>
          </a-upload>
          <!-- 选择文件夹 -->
          <a-upload v-model:file-list="fileList"
                    :auto-upload="false"
                    :show-file-list="false"
                    :directory="true">
            <template #upload-button>
              <a-button type="primary" long>选择文件夹</a-button>
            </template>
          </a-upload>
        </div>
      </a-form-item>
    </a-form>
    <!-- 文件列表 -->
    <a-upload v-if="fileList.length"
              class="file-list-uploader"
              v-model:file-list="fileList"
              :auto-upload="false"
              :show-file-list="true">
      <template #upload-button />
      <template #file-name="{ fileItem }">
        <div class="file-name-wrapper">
          <!-- 文件名称 -->
          <a-tooltip position="left"
                     :mini="true"
                     :auto-fix-position="false"
                     content-class="terminal-tooltip-content"
                     arrow-class="terminal-tooltip-content"
                     :content="fileItem.file.webkitRelativePath || fileItem.file.name">
            <!-- 文件名称 -->
            <span class="file-name text-ellipsis">
              {{ fileItem.file.webkitRelativePath || fileItem.file.name }}
            </span>
          </a-tooltip>
          <!-- 文件大小 -->
          <span class="file-size span-blue">
            {{ getFileSize(fileItem.file.size) }}
          </span>
        </div>
      </template>
    </a-upload>
  </a-modal>
</template>

<script lang="ts">
  export default {
    name: 'sftpUploadModal'
  };
</script>

<script lang="ts" setup>
  import type { FileItem } from '@arco-design/web-vue';
  import type { ITerminalSession } from '@/views/terminal/interfaces';
  import { ref, nextTick } from 'vue';
  import { useTerminalStore } from '@/store';
  import { Message } from '@arco-design/web-vue';
  import { getFileSize } from '@/utils/file';
  import useVisible from '@/hooks/visible';

  const props = defineProps<{
    session?: ITerminalSession;
  }>();
  const emits = defineEmits(['closed']);

  const { visible, setVisible } = useVisible();
  const { transferManager } = useTerminalStore();

  const pathRef = ref();
  const formRef = ref();
  const formModel = ref({
    parentPath: ''
  });
  const fileList = ref<FileItem[]>([]);

  // 打开
  const open = (parent: string) => {
    formModel.value.parentPath = parent;
    setVisible(true);
    nextTick(() => {
      pathRef.value?.focus();
    });
  };

  defineExpose({ open });

  // 确定
  const handleOk = async () => {
    // 验证参数
    const error = await formRef.value.validate();
    if (error) {
      return false;
    }
    if (!fileList.value.length) {
      Message.error('请选择文件');
      return false;
    }
    // 获取上传的文件
    const files = fileList.value.map(s => s.file as File);
    await transferManager.sftp.addUpload(props.session as ITerminalSession, formModel.value.parentPath, files);
    handleClose();
    return true;
  };

  // 关闭
  const handleClose = () => {
    handleClear();
    setVisible(false);
  };

  // 清空
  const handleClear = () => {
    fileList.value = [];
    emits('closed');
  };

</script>

<style lang="less" scoped>
  @file-size-width: 82px;

  .upload-container {
    width: 100%;
    padding: 20px;
  }

  .button-container {
    display: flex;
    width: 100%;
    gap: 12px;

    :deep(.arco-upload) {
      flex: 1;
    }
  }

  .file-list-uploader {
    margin-top: 16px;

    :deep(.arco-upload) {
      display: none;
    }

    :deep(.arco-upload-list) {
      padding: 0 12px 0 0;
      max-height: calc(100vh - 536px);
      overflow-x: hidden;
      overflow-y: auto;
    }

    :deep(.arco-upload-list-item-name) {
      margin-right: 0 !important;
    }

    :deep(.arco-upload-list-item-name-link) {
      width: 100%;
    }

    :deep(.arco-upload-list-item-name-text) {
      width: 100%;
    }

    :deep(.arco-upload-list-item:first-of-type) {
      margin-top: 0 !important;
    }

    :deep(.arco-upload-list-item .arco-upload-progress) {
      display: none;
    }
  }

  .file-name-wrapper {
    display: flex;
    justify-content: space-between;

    .file-name {
      color: var(--color-text-1);
      display: inline-block;
      width: calc(100% - @file-size-width);
    }

    .file-size {
      font-size: 13px;
      display: inline-block;
      width: @file-size-width;
      text-align: end;
    }
  }

</style>
