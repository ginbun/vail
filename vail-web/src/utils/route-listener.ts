import type { RouteLocationNormalized } from 'vue-router';

type Handler = (route: RouteLocationNormalized) => void;
const handlers: Set<Handler> = new Set();

const key = Symbol('ROUTE_CHANGE');

let latestRoute: RouteLocationNormalized;

export function setRouteEmitter(to: RouteLocationNormalized) {
  handlers.forEach(handler => handler(to));
  latestRoute = to;
}

/**
 * 添加路由跳转监听器
 */
export function listenerRouteChange(
  handler: Handler,
  immediate = true
) {
  handlers.add(handler);
  if (immediate && latestRoute) {
    handler(latestRoute);
  }
}

/**
 * 移除路由跳转监听器
 */
export function removeRouteListener() {
  handlers.clear();
}
