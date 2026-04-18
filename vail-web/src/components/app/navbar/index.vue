<template>
  <div class="navbar">
    <!-- 左侧按钮 -->
    <div class="left-side">
      <a-space>
        <!-- LOGO -->
        <img
class="left-side-logo"
             alt="logo"
             draggable="false"
             src="@/assets/logo.svg?url" />
        <!-- 名称 -->
        <a-typography-title
:heading="5"
                            :style="{ margin: 0, fontSize: '18px', height: '1.4em', overflow: 'hidden' }">
          Orion Visor
        </a-typography-title>
        <!-- 收缩菜单 -->
        <icon-menu-fold
v-if="!topMenu && appStore.device === 'mobile'"
                        style="font-size: 22px; cursor: pointer"
                        @click="toggleDrawerMenu" />
      </a-space>
    </div>
    <!-- 顶部菜单 -->
    <div class="center-side">
      <system-menu-tree v-if="topMenu" />
    </div>
    <!-- 右侧操作 -->
    <ul class="right-side">
      <!-- 主机终端 -->
      <li v-permission="['terminal:terminal:access']">
        <a-tooltip content="主机终端">
          <a-button
class="nav-btn"
                    type="outline"
                    shape="circle"
                    @click="openNewRoute({ name: 'terminal' })">
            <template #icon>
              <icon-code-square />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <!-- 切换语言 -->
      <li v-if="false">
        <a-tooltip content="语言">
          <a-button
class="nav-btn"
                    type="outline"
                    shape="circle"
                    @click="setLocalesVisible">
            <template #icon>
              <icon-language />
            </template>
          </a-button>
        </a-tooltip>
        <a-dropdown trigger="click" @select="(s) => changeLocale(s as string)">
          <div ref="localeRef" class="trigger-btn" />
          <template #content>
            <a-doption
v-for="item in locales"
                       :key="item.value"
                       :value="item.value">
              <template #icon>
                <icon-check v-show="item.value === currentLocale" />
              </template>
              {{ item.label }}
            </a-doption>
          </template>
        </a-dropdown>
      </li>
      <!-- 暗色模式 -->
      <li>
        <a-tooltip
:content="theme === 'light'
              ? '点击切换为暗黑模式'
              : '点击切换为亮色模式'">
          <a-button
class="nav-btn"
                    type="outline"
                    shape="circle"
                    @click="handleToggleTheme">
            <template #icon>
              <icon-moon-fill v-if="theme === 'dark'" />
              <icon-sun-fill v-else />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <!-- 系统消息 -->
      <li>
        <a-tooltip content="系统消息" :show-arrow="false">
          <div class="message-box-trigger">
            <a-badge :count="messageCount" dot>
              <a-button
class="nav-btn"
                        type="outline"
                        shape="circle"
                        @click="setMessageBoxVisible">
                <icon-notification />
              </a-button>
            </a-badge>
          </div>
        </a-tooltip>
        <a-popover
trigger="click"
                   content-class="message-popover"
                   position="br"
                   :show-arrow="false"
                   :popup-style="{ marginLeft: '198px' }"
                   :content-style="{ padding: 0, width: '428px' }"
                   @hide="pullHasUnreadMessage">
          <div ref="messageRef" class="ref-btn" />
          <template #content>
            <message-box />
          </template>
        </a-popover>
      </li>
      <!-- 全屏模式 -->
      <li>
        <a-tooltip
:content="isFullscreen
              ? '点击退出全屏模式'
              : '点击切换全屏模式'">
          <a-button
class="nav-btn"
                    type="outline"
                    shape="circle"
                    @click="toggleFullScreen">
            <template #icon>
              <icon-fullscreen-exit v-if="isFullscreen" />
              <icon-fullscreen v-else />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <!-- 刷新页面 -->
      <li>
        <a-tooltip content="刷新页面">
          <a-button
class="nav-btn"
                    type="outline"
                    shape="circle"
                    @click="reloadCurrent">
            <template #icon>
              <icon-refresh />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <!-- 偏好设置 -->
      <li>
        <a-popover :popup-visible="tippedPreference" position="br">
          <template #title>
            💡 点击这里可以修改系统偏好~
          </template>
          <template #content>
           <span style="line-height: 1.8">
             ◾ 可以修改页面布局<br>
             ◾ 可以切换显示视图
           </span>
            <div class="tips-buttons">
              <a-button size="mini" class="mr8" @click="closePreferenceTip(false)">关闭</a-button>
              <a-button size="mini" type="primary" @click="closePreferenceTip(true)">我知道了</a-button>
            </div>
          </template>
          <a-tooltip content="偏好设置">
            <a-button
class="nav-btn"
                      type="outline"
                      shape="circle"
                      @click="openAppSetting">
              <template #icon>
                <icon-settings />
              </template>
            </a-button>
          </a-tooltip>
        </a-popover>
      </li>
      <!-- 用户信息 -->
      <li>
        <a-dropdown trigger="click" position="br">
          <!-- 头像 -->
          <a-avatar
draggable="false"
                    :size="32"
                    :style="{ cursor: 'pointer', backgroundColor: 'rgb(var(--primary-6))', userSelect: 'none' }">
            {{ nickname }}
          </a-avatar>
          <template #content>
            <!-- 个人中心 -->
            <a-doption>
              <a-space @click="router.push({ name: 'userInfo' })">
                <icon-user />
                <span>个人中心</span>
              </a-space>
            </a-doption>
            <!-- 修改密码 -->
            <a-doption>
              <a-space @click="() => updatePasswordRef.open()">
                <icon-lock />
                <span>修改密码</span>
              </a-space>
            </a-doption>
            <!-- 退出登录 -->
            <a-doption>
              <a-space @click="handleLogout">
                <icon-export />
                <span>退出登录</span>
              </a-space>
            </a-doption>
          </template>
        </a-dropdown>
      </li>
    </ul>
    <!-- 修改密码模态框-->
    <update-password-modal ref="updatePasswordRef" @updated="handleLogout" />
  </div>
</template>

<script lang="ts" setup>
  import { computed, inject, onMounted, onUnmounted, ref } from 'vue';
  import useLocale from '@/hooks/locale';
  import useUser from '@/hooks/user';
  import { useRoute, useRouter } from 'vue-router';
  import { useDark, useFullscreen, useToggle } from '@vueuse/core';
  import { useAppStore, useTabBarStore, useTipsStore, useUserStore } from '@/store';
  import { LOCALE_OPTIONS } from '@/locale';
  import { triggerMouseEvent } from '@/utils/event';
  import { openAppSettingKey, toggleDrawerMenuKey } from '@/types/symbol';
  import { preferenceTipsKey } from './const';
  import { getRouteTag, openNewRoute } from '@/router';
  import { REDIRECT_ROUTE_NAME } from '@/router/constants';
  import { checkHasUnreadMessage } from '@/api/system/message';
  import SystemMenuTree from '@/components/system/menu/tree/index.vue';
  import MessageBox from '@/components/system/message-box/index.vue';
  import UpdatePasswordModal from '@/components/user/user/update-password-modal/index.vue';

  const tipsStore = useTipsStore();
  const appStore = useAppStore();
  const userStore = useUserStore();
  const tabBarStore = useTabBarStore();
  const route = useRoute();
  const router = useRouter();
  const { logout } = useUser();
  const { changeLocale, currentLocale } = useLocale();
  const { isFullscreen, toggle: toggleFullScreen } = useFullscreen();

  // 主题
  const darkTheme = useDark({
    selector: 'body',
    attribute: 'arco-theme',
    valueDark: 'dark',
    valueLight: 'light',
    storageKey: 'arco-theme',
    onChanged(dark: boolean) {
      appStore.updateSettings({
        theme: dark ? 'dark' : 'light'
      });
    },
  });

  // 用户名
  const nickname = computed(() => userStore.nickname?.substring(0, 1));
  // 是否展示顶部菜单
  const topMenu = computed(() => appStore.topMenu && appStore.menu);
  // 当前主题
  const theme = computed(() => appStore.theme);

  const locales = [...LOCALE_OPTIONS];
  // 偏好提示
  const tippedPreference = ref(tipsStore.isNotTipped(preferenceTipsKey));
  // 修改密码
  const updatePasswordRef = ref();
  // 消息
  const messageRef = ref();
  // 语言
  const localeRef = ref();
  // 消息数量
  const messageCount = ref(0);
  const messageIntervalId = ref();

  // 打开应用设置
  const openAppSetting = inject(openAppSettingKey) as () => void;

  // 注入收缩菜单
  const toggleDrawerMenu = inject(toggleDrawerMenuKey) as () => void;

  // 切换主题
  const handleToggleTheme = () => {
    useToggle(darkTheme)();
  };

  // 打开消息
  const setMessageBoxVisible = () => {
    triggerMouseEvent(messageRef);
  };

  // 打开语言切换
  const setLocalesVisible = () => {
    triggerMouseEvent(localeRef);
  };

  // 刷新页面
  const reloadCurrent = async () => {
    if (appStore.tabBar) {
      // 重新加载 tab
      const tag = getRouteTag(route);
      tabBarStore.deleteCache(tag);
      await router.push({
        name: REDIRECT_ROUTE_NAME,
        params: { path: route.fullPath },
      });
      tabBarStore.addCache(tag.name);
    } else {
      // 刷新页面
      router.go(0);
    }
  };

  // 退出登录
  const handleLogout = async () => {
    await logout();
  };

  // 获取是否有未读的消息
  const pullHasUnreadMessage = () => {
    // 查询
    checkHasUnreadMessage().then(({ data }) => {
      messageCount.value = data ? 1 : 0;
    });
  };

  // 关闭偏好提示
  const closePreferenceTip = (ack: boolean) => {
    tippedPreference.value = false;
    if (ack) {
      tipsStore.setTipped(preferenceTipsKey);
    }
  };

  defineExpose({ setMessageBoxVisible });

  onMounted(() => {
    // 查询未读消息
    pullHasUnreadMessage();
    // 注册未读消息轮询
    messageIntervalId.value = window.setInterval(pullHasUnreadMessage, 30000);
  });

  onUnmounted(() => {
    // 清理消息轮询
    clearInterval(messageIntervalId.value);
  });

</script>

<style lang="less" scoped>
  .navbar {
    display: flex;
    justify-content: space-between;
    height: 100%;
    background-color: var(--color-bg-2);
    border-bottom: 1px solid var(--color-border);
  }

  .left-side {
    user-select: none;
    display: flex;
    align-items: center;
    padding-left: 20px;
    color: var(--color-text-1);

    &-logo {
      width: 32px;
      height: 32px;
    }
  }

  .center-side {
    flex: 1;
  }

  .right-side {
    display: flex;
    list-style: none;

    :deep(.locale-select) {
      border-radius: 20px;
    }

    li {
      display: flex;
      align-items: center;
      padding: 0 10px;
    }

    a {
      color: var(--color-text-1);
      text-decoration: none;
    }

    .nav-btn {
      border-color: rgb(var(--gray-2));
      color: rgb(var(--gray-8));
      font-size: 16px;
    }

    .trigger-btn,
    .ref-btn {
      position: absolute;
      bottom: 14px;
    }

    .trigger-btn {
      margin-left: 14px;
    }
  }
</style>

<style lang="less">
  .message-popover {
    .arco-popover-content {
      margin-top: 0;
    }
  }

  .tips-buttons {
    margin-top: 12px;
    display: flex;
    justify-content: flex-end;
  }
</style>
