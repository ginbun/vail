# 终端字体选择实现说明

## 概述

终端字体选择功能采用**前后端结合**的方式实现：

- **后端**: 提供字体列表数据（存储在数据库字典表中）
- **前端**: 实现字体选择器 UI，应用字体到终端，保存用户偏好

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                     字体选择流程                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. 后端提供字体列表                                          │
│     ┌──────────────────────────────────────┐                │
│     │ 数据库 (sys_dict_value)              │                │
│     │ - terminalFontFamily                 │                │
│     │ - 默认、Consolas、Fira Code...       │                │
│     └──────────────────────────────────────┘                │
│                    ↓                                          │
│     ┌──────────────────────────────────────┐                │
│     │ API: /api/infra/dict-value/list      │                │
│     │ 返回: [{value, label}]               │                │
│     └──────────────────────────────────────┘                │
│                                                               │
│  2. 前端加载字体列表                                          │
│     ┌──────────────────────────────────────┐                │
│     │ Vue 组件                              │                │
│     │ - 下拉选择器 (a-select)              │                │
│     │ - 支持自定义输入                      │                │
│     │ - 实时预览字体效果                    │                │
│     └──────────────────────────────────────┘                │
│                    ↓                                          │
│  3. 应用字体到终端                                            │
│     ┌──────────────────────────────────────┐                │
│     │ xterm.js                              │                │
│     │ terminal.options.fontFamily = ...     │                │
│     └──────────────────────────────────────┘                │
│                    ↓                                          │
│  4. 保存用户偏好                                              │
│     ┌──────────────────────────────────────┐                │
│     │ 用户偏好设置 (preference)            │                │
│     │ sshDisplaySetting.fontFamily         │                │
│     └──────────────────────────────────────┘                │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## 数据库设计

### 字典键 (sys_dict_key)

```sql
INSERT INTO sys_dict_key (key_name, value_type, description)
VALUES ('terminalFontFamily', 'STRING', '终端字体样式');
```

### 字典值 (sys_dict_value)

```sql
-- 默认字体（使用系统默认）
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, '_', '_', '默认', 10);

-- Consolas
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Consolas', 'Consolas', 'Consolas', 50);

-- Fira Code (支持连字)
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Fira Code', 'Fira Code', 'Fira Code', 60);

-- JetBrains Mono
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'JetBrains Mono', 'JetBrains Mono', 'JetBrains Mono', 70);

-- Source Code Pro
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Source Code Pro', 'Source Code Pro', 'Source Code Pro', 80);

-- Cascadia Mono (Windows Terminal 默认)
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Cascadia Mono', 'Cascadia Mono', 'Cascadia Mono', 50);

-- Courier New (经典等宽字体)
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Courier New', 'Courier New', 'Courier New', 20);

-- Lucida Console
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Lucida Console', 'Lucida Console', 'Lucida Console', 30);

-- Courier
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES (21, 'Courier', 'Courier', 'Courier', 40);
```

## 前端实现

### 1. 字体选择器组件

**文件**: `terminal-ssh-display-block.vue`

```vue
<template>
  <a-form-item field="fontFamily" label="字体样式">
    <a-select 
      v-model="formModel.fontFamily"
      placeholder="请选择字体样式"
      :options="toOptions(fontFamilyKey)"
      :allow-create="true"
      :filter-option="labelFilter"
    >
      <!-- 下拉选项预览 -->
      <template #label="{ data }">
        <span :style="{ 
          fontFamily: data.value === '_' 
            ? defaultFontFamily 
            : data.value 
        }">
          {{ data.label }}
        </span>
      </template>
      
      <!-- 选项列表预览 -->
      <template #option="{ data }">
        <span :style="{ 
          fontFamily: data.value === '_' 
            ? defaultFontFamily 
            : data.value 
        }">
          {{ data.label }}
        </span>
      </template>
    </a-select>
  </a-form-item>
</template>

<script setup lang="ts">
import { useDictStore } from '@/store';
import { fontFamilyKey } from '@/views/terminal/types/const';
import { defaultFontFamily } from '@/types/xterm';

const dictStore = useDictStore();

// 将字典值转换为选项
const toOptions = (key: string) => {
  return dictStore.getDict(key);
};
</script>
```

**关键特性**:
- ✅ 支持从字典加载字体列表
- ✅ 支持自定义输入字体名称 (`:allow-create="true"`)
- ✅ 实时预览字体效果 (`:style="{ fontFamily: ... }"`)
- ✅ 支持搜索过滤 (`:filter-option="labelFilter"`)

### 2. 应用字体到终端

**文件**: `ssh-session.ts`

```typescript
import { Terminal } from '@xterm/xterm';
import { defaultFontFamily } from '@/types/xterm';

// 默认字体回退链
const defaultFontFamily = 'Consolas, Courier New, Monaco, courier, monospace';

// 初始化终端时应用字体
const fontFamily = preference.sshDisplaySetting.fontFamily;

this.inst.options = {
  // 如果选择了默认 (_)，使用默认字体链
  // 否则将用户选择的字体放在最前面，后面跟默认字体作为回退
  fontFamily: fontFamily === '_' 
    ? defaultFontFamily 
    : `${fontFamily}, ${defaultFontFamily}`,
  fontSize: preference.sshDisplaySetting.fontSize,
  fontWeight: preference.sshDisplaySetting.fontWeight,
  // ... 其他配置
};
```

**字体回退机制**:
```
用户选择: Fira Code
实际应用: "Fira Code, Consolas, Courier New, Monaco, courier, monospace"

如果 Fira Code 不可用 → 使用 Consolas
如果 Consolas 不可用 → 使用 Courier New
... 依此类推
```

### 3. 保存用户偏好

**文件**: `terminal/store`

```typescript
interface TerminalSshDisplaySetting {
  fontFamily?: string;      // 字体样式
  fontSize?: number;         // 字体大小
  lineHeight?: number;       // 行高
  letterSpacing?: number;    // 字间距
  fontWeight?: string;       // 普通文本字重
  fontWeightBold?: string;   // 加粗文本字重
  cursorStyle?: string;      // 光标样式
}

// 保存偏好到后端
await updateTerminalPreference(
  TerminalPreferenceItem.SSH_DISPLAY_SETTING,
  formModel.value
);
```

## API 接口

### 获取字体列表

**请求**:
```http
GET /api/infra/dict-value/list?keyName=terminalFontFamily
```

**响应**:
```json
[
  {
    "id": 129,
    "keyId": 21,
    "name": "_",
    "value": "_",
    "label": "默认",
    "extra": "{}",
    "sort": 10
  },
  {
    "id": 130,
    "keyId": 21,
    "name": "Courier New",
    "value": "Courier New",
    "label": "Courier New",
    "extra": "{}",
    "sort": 20
  },
  {
    "id": 131,
    "keyId": 21,
    "name": "Fira Code",
    "value": "Fira Code",
    "label": "Fira Code",
    "extra": "{}",
    "sort": 60
  }
  // ... 更多字体
]
```

### 保存用户偏好

**请求**:
```http
PUT /api/infra/preference
Content-Type: application/json

{
  "type": "TERMINAL",
  "item": "SSH_DISPLAY_SETTING",
  "value": {
    "fontFamily": "Fira Code",
    "fontSize": 14,
    "lineHeight": 1.2,
    "fontWeight": "normal",
    "fontWeightBold": "bold"
  }
}
```

## 推荐字体列表

### 编程专用字体（支持连字）

1. **Fira Code** ⭐ 推荐
   - 特点: 支持连字 (ligatures)，`!=` 显示为 `≠`
   - 下载: https://github.com/tonsky/FiraCode

2. **JetBrains Mono** ⭐ 推荐
   - 特点: 专为开发者设计，清晰易读
   - 下载: https://www.jetbrains.com/lp/mono/

3. **Cascadia Code**
   - 特点: Windows Terminal 默认字体，支持连字
   - 下载: https://github.com/microsoft/cascadia-code

4. **Source Code Pro**
   - 特点: Adobe 出品，优雅的等宽字体
   - 下载: https://github.com/adobe-fonts/source-code-pro

### 经典等宽字体

5. **Consolas** (Windows 内置)
   - 特点: 清晰、易读、广泛使用

6. **Monaco** (macOS 内置)
   - 特点: macOS 经典终端字体

7. **Courier New** (跨平台)
   - 特点: 经典打字机风格

8. **Lucida Console** (Windows 内置)
   - 特点: 简洁、清晰

## 实现步骤（Vail 项目）

### 1. 添加数据库迁移

创建 `vail-rs/migrations/0004_terminal_fonts.sql`:

```sql
-- 添加终端字体字典键
INSERT INTO sys_dict_key (key_name, value_type, description, creator, updater, create_time, update_time)
VALUES ('terminalFontFamily', 'STRING', '终端字体样式', 'system', 'system', NOW(), NOW());

-- 添加字体选项
DO $
DECLARE
    font_key_id BIGINT;
BEGIN
    SELECT id INTO font_key_id FROM sys_dict_key WHERE key_name = 'terminalFontFamily';

    -- 默认字体
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, '_', '_', '默认', '{}', 10, 'system', 'system', NOW(), NOW(), 0);

    -- Consolas
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Consolas', 'Consolas', 'Consolas', '{}', 50, 'system', 'system', NOW(), NOW(), 0);

    -- Fira Code
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Fira Code', 'Fira Code', 'Fira Code', '{}', 60, 'system', 'system', NOW(), NOW(), 0);

    -- JetBrains Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'JetBrains Mono', 'JetBrains Mono', 'JetBrains Mono', '{}', 70, 'system', 'system', NOW(), NOW(), 0);

    -- Source Code Pro
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Source Code Pro', 'Source Code Pro', 'Source Code Pro', '{}', 80, 'system', 'system', NOW(), NOW(), 0);

    -- Cascadia Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Cascadia Mono', 'Cascadia Mono', 'Cascadia Mono', '{}', 90, 'system', 'system', NOW(), NOW(), 0);

    -- Courier New
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Courier New', 'Courier New', 'Courier New', '{}', 20, 'system', 'system', NOW(), NOW(), 0);

    -- Lucida Console
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Lucida Console', 'Lucida Console', 'Lucida Console', '{}', 30, 'system', 'system', NOW(), NOW(), 0);

    -- Courier
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Courier', 'Courier', 'Courier', '{}', 40, 'system', 'system', NOW(), NOW(), 0);

    -- Monaco
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_key_id, 'Monaco', 'Monaco', 'Monaco', '{}', 100, 'system', 'system', NOW(), NOW(), 0);

END $;
```

### 2. 前端实现（vail-web）

**字体选择器组件**:

```vue
<template>
  <div class="font-selector">
    <label>字体样式</label>
    <select v-model="selectedFont" @change="applyFont">
      <option 
        v-for="font in fonts" 
        :key="font.value" 
        :value="font.value"
        :style="{ fontFamily: font.value === '_' ? defaultFontFamily : font.value }"
      >
        {{ font.label }}
      </option>
    </select>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Terminal } from '@xterm/xterm';

const defaultFontFamily = 'Consolas, Courier New, Monaco, courier, monospace';
const fonts = ref([]);
const selectedFont = ref('_');

onMounted(async () => {
  // 加载字体列表
  const response = await fetch('/api/infra/dict-value/list?keyName=terminalFontFamily');
  fonts.value = await response.json();
});

function applyFont() {
  const fontFamily = selectedFont.value === '_' 
    ? defaultFontFamily 
    : `${selectedFont.value}, ${defaultFontFamily}`;
  
  // 应用到终端
  terminal.value.options.fontFamily = fontFamily;
}
</script>
```

## 字体大小和字重

除了字体样式，还支持配置：

### 字体大小 (terminalFontSize)

```sql
INSERT INTO sys_dict_key (key_name, value_type, description)
VALUES ('terminalFontSize', 'INTEGER', '终端字体大小');

-- 10px - 20px
INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES 
  (22, '10', '10', '10px', 10),
  (22, '11', '11', '11px', 20),
  (22, '12', '12', '12px', 30),
  (22, '13', '13', '13px', 40),
  (22, '14', '14', '14px', 50),
  (22, '15', '15', '15px', 60),
  (22, '16', '16', '16px', 70),
  (22, '18', '18', '18px', 80),
  (22, '20', '20', '20px', 90);
```

### 字体粗细 (terminalFontWeight)

```sql
INSERT INTO sys_dict_key (key_name, value_type, description)
VALUES ('terminalFontWeight', 'STRING', '终端文本粗细');

INSERT INTO sys_dict_value (key_id, name, value, label, sort)
VALUES 
  (23, 'normal', 'normal', '正常', 10),
  (23, 'bold', 'bold', '加粗', 20),
  (23, '100', '100', '极细', 30),
  (23, '300', '300', '细', 40),
  (23, '500', '500', '中等', 50),
  (23, '700', '700', '粗', 60),
  (23, '900', '900', '极粗', 70);
```

## 总结

字体选择功能的实现方式：

1. **后端**: 
   - ✅ 数据库存储字体列表（字典表）
   - ✅ 提供 API 接口获取字体列表
   - ✅ 保存用户字体偏好

2. **前端**:
   - ✅ 下拉选择器展示字体列表
   - ✅ 实时预览字体效果
   - ✅ 支持自定义输入字体名称
   - ✅ 应用字体到 xterm.js 终端
   - ✅ 字体回退机制保证兼容性

3. **用户体验**:
   - ✅ 所见即所得的字体预览
   - ✅ 支持搜索和过滤
   - ✅ 自动保存用户偏好
   - ✅ 跨设备同步设置

这种设计既保证了灵活性（可以轻松添加新字体），又保证了用户体验（实时预览、自定义输入）。
