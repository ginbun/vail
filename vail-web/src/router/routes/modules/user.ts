import type { AppRouteRecordRaw } from '../types';
import { DEFAULT_LAYOUT } from '../base';

const USER: AppRouteRecordRaw = {
  name: 'userModule',
  path: '/user-module',
  component: DEFAULT_LAYOUT,
  children: [
    {
      name: 'userRole',
      path: '/user/role',
      component: () => import('@/views/user/role/index.vue'),
    },
    {
      name: 'userList',
      path: '/user/list',
      component: () => import('@/views/user/user/index.vue'),
    },
    {
      name: 'userInfo',
      path: '/user/info',
      component: () => import('@/views/user/info/index.vue'),
    },
    {
      name: 'operatorLog',
      path: '/user/operator-log',
      component: () => import('@/views/user/operator-log/index.vue'),
    },
    {
      name: 'userSession',
      path: '/user/session',
      component: () => import('@/views/user/user-session/index.vue'),
    },
    {
      name: 'lockedUser',
      path: '/user/locked',
      component: () => import('@/views/user/locked-user/index.vue'),
    },
  ],
};

export default USER;
