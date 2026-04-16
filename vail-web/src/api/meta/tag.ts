import type { TableData } from '@arco-design/web-vue';
import type { DataGrid, Pagination, OrderDirection } from '@/types/global';
import axios from 'axios';

export type TagType = 'HOST' | 'MONITOR_DASH' | string

/**
 * tag 创建对象
 */
export interface TagCreateRequest {
  name?: string;
  type?: TagType;
}

/**
 * tag 修改对象
 */
export interface TagUpdateRequest extends TagCreateRequest {
  id?: number;
}

/**
 * tag 查询请求
 */
export interface TagQueryRequest extends Pagination, OrderDirection {
  name?: string;
  type?: string;
}

/**
 * tag 响应对象
 */
export interface TagQueryResponse extends TableData, TagItem {
  relCount: string;
  createTime: number;
  updateTime: number;
  creator: string;
  updater: string;
}

/**
 * tag 元素
 */
export interface TagItem {
  id: number;
  name: string;
}

/**
 * 创建标签
 */
export function createTag(request: TagCreateRequest) {
  return axios.post('/infra/tag/create', request);
}

/**
 * 修改标签
 */
export function updateTag(request: TagUpdateRequest) {
  return axios.put('/infra/tag/update', request);
}

/**
 * 分页查询标签
 */
export function getTagPage(request: TagQueryRequest) {
  return axios.post<DataGrid<TagQueryResponse>>('/infra/tag/query', request);
}

/**
 * 查询标签
 */
export function getTagList(type: TagType) {
  return axios.get<Array<TagItem>>('/infra/tag/list', { params: { type } });
}

/**
 * 通过 id 删除标签
 */
export function deleteTag(id: number) {
  return axios.delete('/infra/tag/delete', { params: { id } });
}
