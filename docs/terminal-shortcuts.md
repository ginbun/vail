# 终端快捷键配置实现说明

## 概述

终端快捷键配置采用**纯前端实现 + 后端存储偏好**的方式：

- **前端**: 负责快捷键监听、触发、UI 配置界面（95%）
- **后端**: 仅负责存储用户的快捷键偏好设置（5%）

**关键特点**: 快捷键的定义、监听、触发逻辑**完全在前端实现**，后端只是一个"存储服务"。

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                   快捷键实现架构                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  前端 (定义层) - 硬编码在前端代码中                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ TerminalShortcutItems (const.ts)                      │   │
│  │                                                        │   │
│  │ 预定义 23 个快捷键操作:                                │   │
│  │ ├─ 全局快捷键 (9个)                                   │   │
│  │ │  └─ 切换 tab, 打开命令片段, 截图...                │   │
│  │ ├─ 会话快捷键 (5个)                                   │   │
│  │ │  └─ 复制会话, 关闭会话, 切换会话...                │   │
│  │ └─ 终端快捷键 (9个)                                   │   │
│  │    └─ 复制, 粘贴, 搜索, 上传文件...                  │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  前端 (配置层) - 用户自定义快捷键绑定                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 快捷键设置界面                                         │   │
│  │                                                        │   │
│  │ 用户可以为每个操作配置:                                │   │
│  │ ├─ Ctrl + Shift + Alt 组合                           │   │
│  │ ├─ 按键代码 (KeyC, KeyW, ArrowUp...)                 │   │
│  │ └─ 启用/禁用状态                                       │   │
│  │                                                        │   │
│  │ 示例: "关闭 tab" → Ctrl + Shift + W                  │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  前端 (监听层) - 全局键盘事件监听                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ window.addEventListener('keydown', handler)           │   │
│  │                                                        │   │
│  │ 1. 捕获键盘事件                                        │   │
│  │ 2. 匹配用户配置的快捷键                                │   │
│  │ 3. 触发对应的操作                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  前端 (执行层) - 执行快捷键对应的操作                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 根据快捷键类型执行不同操作:                            │   │
│  │                                                        │   │
│  │ - 全局: 切换 tab, 打开面板...                         │   │
│  │ - 会话: 复制/关闭会话...                              │   │
│  │ - 终端: terminal.selectAll(), terminal.paste()...    │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  后端 (存储层) - 仅存储用户偏好                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ PUT /api/infra/preference                             │   │
│  │                                                        │   │
│  │ {                                                      │   │
│  │   "type": "TERMINAL",                                 │   │
│  │   "item": "SHORTCUT_SETTING",                         │   │
│  │   "value": {                                          │   │
│  │     "enabled": true,                                  │   │
│  │     "keys": [                                         │   │
│  │       {                                               │   │
│  │         "item": "closeTab",                           │   │
│  │         "ctrlKey": true,                              │   │
│  │         "shiftKey": true,                             │   │
│  │         "altKey": false,                              │   │
│  │         "code": "KeyW",                               │   │
│  │         "enabled": true                               │   │
│  │       }                                               │   │
│  │     ]                                                 │   │
│  │   }                                                   │   │
│  │ }                                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## 快捷键分类

### 1. 全局快捷键 (GLOBAL) - 9 个

作用于整个终端应用，不限于特定会话。

| 操作 | 默认快捷键 | 说明 |
|------|-----------|------|
| 切换到前一个 tab | `Ctrl + Shift + Alt + [` | 切换到左侧 tab |
| 切换到后一个 tab | `Ctrl + Shift + Alt + ]` | 切换到右侧 tab |
| 关闭当前 tab | `Ctrl + Shift + Alt + W` | 关闭当前 tab |
| 打开新建连接 tab | `Ctrl + Shift + Alt + N` | 打开新建连接页面 |
| 打开命令片段 | `Ctrl + Shift + Alt + C` | 打开命令片段面板 |
| 打开书签路径 | `Ctrl + Shift + Alt + P` | 打开路径书签面板 |
| 打开文件传输列表 | `Ctrl + Shift + Alt + T` | 打开文件传输列表 |
| 打开发送命令 | `Ctrl + Shift + Alt + I` | 打开命令输入框 |
| 截图 | `Ctrl + Shift + Alt + S` | 截取终端屏幕 |

### 2. 会话快捷键 (SESSION) - 5 个

作用于当前活动的终端会话。

| 操作 | 默认快捷键 | 说明 |
|------|-----------|------|
| 打开新建连接弹框 | `Ctrl + Alt + N` | 在当前 tab 新建连接 |
| 复制会话 | `Ctrl + Alt + O` | 复制当前会话 |
| 关闭会话 | `Ctrl + Alt + W` | 关闭当前会话 |
| 切换到前一个会话 | `Ctrl + Alt + [` | 切换到左侧会话 |
| 切换到后一个会话 | `Ctrl + Alt + ]` | 切换到右侧会话 |

### 3. 终端快捷键 (TERMINAL) - 9 个

作用于 xterm.js 终端实例本身。

| 操作 | 默认快捷键 | 说明 |
|------|-----------|------|
| 复制 | `Ctrl + Shift + C` | 复制选中文本 |
| 粘贴 | `Ctrl + Shift + Insert` | 粘贴文本 |
| 去顶部 | `Ctrl + Shift + ↑` | 滚动到顶部 |
| 去底部 | `Ctrl + Shift + ↓` | 滚动到底部 |
| 全选 | `Ctrl + Shift + A` | 全选终端内容 |
| 搜索 | `Ctrl + Shift + F` | 打开搜索框 |
| 上传文件 | `Ctrl + Shift + U` | 打开文件上传 |
| 增大字号 | `Ctrl + Alt + =` | 增大字体大小 |
| 减小字号 | `Ctrl + Alt + -` | 减小字体大小 |

## 数据结构

### 快捷键配置对象

```typescript
interface TerminalShortcutSetting {
  enabled: boolean;                    // 是否启用快捷键
  keys: Array<TerminalShortcutKey>;   // 快捷键列表
}

interface TerminalShortcutKey {
  item: string;        // 操作标识 (如 "closeTab")
  ctrlKey: boolean;    // 是否按下 Ctrl
  shiftKey: boolean;   // 是否按下 Shift
  altKey: boolean;     // 是否按下 Alt
  code: string;        // 按键代码 (如 "KeyW", "ArrowUp")
  enabled: boolean;    // 是否启用此快捷键
}
```

### 按键代码 (KeyCode)

```typescript
// 字母键
"KeyA" - "KeyZ"

// 数字键
"Digit0" - "Digit9"

// 方向键
"ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"

// 特殊键
"Enter", "Space", "Backspace", "Delete", "Tab", "Escape"
"Insert", "Home", "End", "PageUp", "PageDown"

// 符号键
"Minus" (-), "Equal" (=), "BracketLeft" ([), "BracketRight" (])
"Backslash" (\), "Semicolon" (;), "Quote" ('), "Comma" (,)
"Period" (.), "Slash" (/), "Backquote" (`)

// 功能键
"F1" - "F12"
```

## 前端实现

### 1. 快捷键定义 (硬编码)

**文件**: `src/views/terminal/types/const.ts`

```typescript
// 快捷键操作类型
export const TerminalShortcutType = {
  GLOBAL: 1,      // 全局快捷键
  SESSION: 3,     // 会话快捷键
  TERMINAL: 4     // 终端快捷键
};

// 快捷键操作标识
export const TerminalShortcutKeys = {
  CLOSE_TAB: 'closeTab',
  CHANGE_TO_PREV_TAB: 'changeToPrevTab',
  CHANGE_TO_NEXT_TAB: 'changeToNextTab',
  // ... 更多操作
};

// 快捷键列表定义
export const TerminalShortcutItems: Array<ShortcutKeyItem> = [
  {
    item: 'closeTab',
    content: '关闭当前 tab',
    type: TerminalShortcutType.GLOBAL
  },
  {
    item: 'copy',
    content: '复制',
    type: TerminalShortcutType.TERMINAL
  },
  // ... 23 个快捷键定义
];
```

### 2. 快捷键配置界面

**文件**: `terminal-shortcut-setting.vue`

```vue
<template>
  <div class="shortcut-setting">
    <!-- 启用/禁用开关 -->
    <a-switch v-model="enabled" />
    
    <!-- 快捷键列表 -->
    <div v-for="key in shortcutKeys" :key="key.item">
      <span>{{ key.content }}</span>
      
      <!-- 快捷键输入框 -->
      <input 
        v-if="key.editable"
        :value="key.shortcutKey"
        @keydown="captureShortcut($event, key)"
        placeholder="请按下快捷键"
        readonly
      />
      
      <!-- 显示当前快捷键 -->
      <span v-else>{{ key.shortcutKey }}</span>
      
      <!-- 编辑/启用按钮 -->
      <button @click="editShortcut(key)">编辑</button>
      <a-switch v-model="key.enabled" />
    </div>
    
    <!-- 保存按钮 -->
    <button @click="savePreference">保存</button>
  </div>
</template>

<script setup lang="ts">
// 捕获快捷键
const captureShortcut = (e: KeyboardEvent, key: TerminalShortcutKey) => {
  e.preventDefault();
  
  // 记录修饰键
  key.ctrlKey = e.ctrlKey;
  key.shiftKey = e.shiftKey;
  key.altKey = e.altKey;
  
  // 记录按键代码
  if (e.key !== 'Control' && e.key !== 'Shift' && e.key !== 'Alt') {
    key.code = e.code;
  }
  
  // 计算显示文本
  key.shortcutKey = computeShortcutKey(key);
};

// 计算快捷键显示文本
const computeShortcutKey = (key: TerminalShortcutKey): string => {
  const parts = [];
  if (key.ctrlKey) parts.push('Ctrl');
  if (key.altKey) parts.push('Alt');
  if (key.shiftKey) parts.push('Shift');
  
  // 转换按键代码为可读文本
  let code = key.code;
  if (code.startsWith('Key')) {
    code = code.substring(3);  // "KeyW" → "W"
  } else if (code.startsWith('Digit')) {
    code = code.substring(5);  // "Digit1" → "1"
  } else if (code === 'ArrowUp') {
    code = '↑';
  }
  // ... 更多转换
  
  parts.push(code);
  return parts.join(' + ');  // "Ctrl + Shift + W"
};
</script>
```

### 3. 快捷键监听和触发

**文件**: `main-content.vue` / `terminal-panels-view.vue`

```typescript
import { addEventListen, removeEventListen } from '@/utils/event';

// 监听键盘事件
onMounted(() => {
  if (preference.shortcutSetting.enabled) {
    addEventListen(window, 'keydown', handleKeyboard);
  }
});

// 处理键盘事件
const handleKeyboard = (event: Event) => {
  const e = event as KeyboardEvent;
  
  // 查找匹配的快捷键
  const key = preference.shortcutSetting.keys.find(key => {
    return key.code === e.code
      && key.altKey === e.altKey
      && key.shiftKey === e.shiftKey
      && key.ctrlKey === e.ctrlKey
      && key.enabled;
  });
  
  if (!key) return;
  
  // 阻止默认行为
  event.preventDefault();
  event.stopPropagation();
  
  // 执行对应操作
  executeShortcutAction(key.item);
};

// 执行快捷键操作
const executeShortcutAction = (action: string) => {
  switch (action) {
    case 'closeTab':
      tabManager.closeCurrentTab();
      break;
    case 'copy':
      terminal.selectAll();
      document.execCommand('copy');
      break;
    case 'paste':
      navigator.clipboard.readText().then(text => {
        terminal.paste(text);
      });
      break;
    // ... 更多操作
  }
};

// 移除监听
onUnmounted(() => {
  if (preference.shortcutSetting.enabled) {
    removeEventListen(window, 'keydown', handleKeyboard);
  }
});
```

### 4. 终端快捷键处理

**文件**: `ssh-session.ts`

```typescript
// 在终端实例上注册快捷键
private registerShortcut(preference: TerminalPreference) {
  this.inst.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    // 检查是否为内置快捷键 (Ctrl+C, Ctrl+V 等)
    if (this.handler.checkIsBuiltin(e)) {
      return true;  // 允许默认行为
    }
    
    // 检查是否为自定义快捷键
    if (preference.shortcutSetting.enabled) {
      const shortcutKey = this.handler.getShortcutKey(e);
      
      if (shortcutKey?.type === TerminalShortcutType.TERMINAL) {
        // 执行终端快捷键操作
        this.handler.invokeHandle(shortcutKey.item);
        return false;  // 阻止默认行为
      }
    }
    
    return true;  // 允许默认行为
  });
}
```

## 后端实现

### 1. 默认快捷键配置

**文件**: `TerminalPreferenceStrategy.java`

```java
private String getDefaultShortcutSetting() {
    return TerminalPreferenceModel.ShortcutSettingModel.builder()
        .enabled(true)
        .keys(Lists.of(
            // 全局快捷键
            new ShortcutKeysModel("closeTab", true, true, true, "KeyW", true),
            new ShortcutKeysModel("changeToPrevTab", true, true, true, "BracketLeft", true),
            // ... 更多快捷键
            
            // 会话快捷键
            new ShortcutKeysModel("copySession", true, false, true, "KeyO", true),
            // ... 更多快捷键
            
            // 终端快捷键
            new ShortcutKeysModel("copy", true, true, false, "KeyC", true),
            new ShortcutKeysModel("paste", true, true, false, "Insert", true)
            // ... 更多快捷键
        ))
        .build()
        .toJsonString();
}
```

### 2. 用户偏好存储

**API**: `PUT /api/infra/preference`

```json
{
  "type": "TERMINAL",
  "item": "SHORTCUT_SETTING",
  "value": {
    "enabled": true,
    "keys": [
      {
        "item": "closeTab",
        "ctrlKey": true,
        "shiftKey": true,
        "altKey": false,
        "code": "KeyW",
        "enabled": true
      }
    ]
  }
}
```

## 关键特性

### 1. 实时捕获

用户在配置界面按下任意键盘组合，系统立即捕获并显示：

```
用户按下: Ctrl + Shift + W
显示: Ctrl + Shift + W
存储: { ctrlKey: true, shiftKey: true, altKey: false, code: "KeyW" }
```

### 2. 冲突检测

系统会检测快捷键是否与浏览器内置快捷键冲突：

```typescript
// 浏览器内置快捷键 (不可覆盖)
const builtinKeys = [
  { code: 'KeyT', ctrlKey: true },      // Ctrl+T (新标签页)
  { code: 'KeyW', ctrlKey: true },      // Ctrl+W (关闭标签页)
  { code: 'KeyR', ctrlKey: true },      // Ctrl+R (刷新)
  // ... 更多
];
```

### 3. 安全环境检测

某些快捷键只在 HTTPS 或 localhost 环境下可用：

```typescript
const isSecureEnvironment = 
  window.location.protocol === 'https:' || 
  window.location.hostname === 'localhost';

if (!isSecureEnvironment) {
  // 显示警告: 某些快捷键在非安全环境下不可用
}
```

### 4. 启用/禁用控制

- **全局开关**: 一键启用/禁用所有快捷键
- **单个开关**: 每个快捷键可以单独启用/禁用

```typescript
// 全局禁用
preference.shortcutSetting.enabled = false;

// 单个禁用
preference.shortcutSetting.keys.find(k => k.item === 'closeTab').enabled = false;
```

## 与主题、字体的关系

| 功能 | 实现方式 | 数据来源 | 存储位置 |
|------|---------|---------|---------|
| **主题** | 前端应用 + 后端提供数据 | 数据库字典表 | `terminalTheme` |
| **字体** | 前端应用 + 后端提供数据 | 数据库字典表 | `terminalFontFamily` |
| **快捷键** | **纯前端实现** | **前端硬编码** | 用户偏好 JSON |

**关键区别**:
- 主题和字体的**可选项列表**存储在数据库中
- 快捷键的**操作定义**硬编码在前端代码中
- 后端只存储用户选择的**配置值**

## 实现步骤（Vail 项目）

### 1. 前端实现（主要工作）

由于快捷键是纯前端实现，需要在 `vail-web` 中完成：

```typescript
// 1. 定义快捷键列表
export const SHORTCUT_ITEMS = [
  { item: 'closeTab', content: '关闭 tab', type: 'GLOBAL' },
  { item: 'copy', content: '复制', type: 'TERMINAL' },
  // ... 23 个快捷键
];

// 2. 创建配置界面
<ShortcutSettings 
  v-model:shortcuts="shortcuts"
  @save="saveToBackend"
/>

// 3. 全局监听键盘事件
window.addEventListener('keydown', (e) => {
  const matched = findMatchingShortcut(e);
  if (matched) {
    executeAction(matched.item);
  }
});

// 4. 保存到后端
await fetch('/api/infra/preference', {
  method: 'PUT',
  body: JSON.stringify({
    type: 'TERMINAL',
    item: 'SHORTCUT_SETTING',
    value: shortcuts
  })
});
```

### 2. 后端实现（存储服务）

后端只需要提供默认配置和存储服务：

```java
// 默认快捷键配置 (可选)
private String getDefaultShortcutSetting() {
    return """
    {
      "enabled": true,
      "keys": [
        {"item": "closeTab", "ctrlKey": true, "shiftKey": true, "code": "KeyW", "enabled": true}
      ]
    }
    """;
}
```

## 总结

快捷键配置的实现方式：

1. **前端主导** (95%)
   - ✅ 快捷键操作定义（硬编码）
   - ✅ 配置界面实现
   - ✅ 键盘事件监听
   - ✅ 快捷键匹配和触发
   - ✅ 操作执行逻辑

2. **后端辅助** (5%)
   - ✅ 提供默认配置
   - ✅ 存储用户偏好
   - ✅ 跨设备同步

**核心理念**: 快捷键是**用户交互逻辑**，应该在前端实现。后端只是一个"云存储"，保存用户的配置选择。

这种设计的优势：
- ✅ 响应速度快（无需请求后端）
- ✅ 离线可用（配置存储在本地）
- ✅ 易于扩展（添加新快捷键只需修改前端）
- ✅ 减轻后端负担（无需处理复杂的快捷键逻辑）
