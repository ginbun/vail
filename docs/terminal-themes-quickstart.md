# 终端主题快速开始

## 快速启动

### 1. 启动 Vail 服务

```bash
# 使用 Docker Compose
docker compose up --build

# 或者直接运行 Rust 后端
cd vail-rs
cargo run
```

服务启动后，数据库迁移会自动运行，终端主题数据会被初始化。

### 2. 验证主题数据

**方法 1: 使用 API**

```bash
curl http://localhost:3000/api/terminal/themes | jq
```

**方法 2: 使用测试脚本**

```bash
cd vail-rs
./scripts/test_themes.sh
```

**方法 3: 直接查询数据库**

```bash
psql -U vail -d vail -f vail-rs/scripts/verify_themes.sql
```

### 3. 在前端使用主题

#### Vue 3 + TypeScript 示例

```typescript
// api/terminal.ts
export interface TerminalTheme {
  name: string;
  dark: boolean;
  schema: {
    background: string;
    foreground: string;
    cursor: string;
    selectionBackground: string;
    black: string;
    red: string;
    green: string;
    yellow: string;
    blue: string;
    cyan: string;
    white: string;
    brightBlack: string;
    brightRed: string;
    brightGreen: string;
    brightYellow: string;
    brightBlue: string;
    brightCyan: string;
    brightWhite: string;
  };
}

export async function getTerminalThemes(): Promise<TerminalTheme[]> {
  const response = await fetch('/api/terminal/themes');
  return response.json();
}
```

#### 应用主题到 xterm.js

```typescript
import { Terminal } from '@xterm/xterm';
import { getTerminalThemes } from '@/api/terminal';

// 获取主题列表
const themes = await getTerminalThemes();

// 找到 Catppuccin Mocha 主题
const mochaTheme = themes.find(t => t.name === 'Catppuccin Mocha');

// 创建终端实例并应用主题
const terminal = new Terminal({
  theme: mochaTheme?.schema,
  fontFamily: 'Cascadia Mono, Consolas, monospace',
  fontSize: 14,
  cursorBlink: true,
});

terminal.open(document.getElementById('terminal-container')!);
```

#### 主题切换组件

```vue
<template>
  <div class="theme-selector">
    <label>选择主题:</label>
    <select v-model="selectedTheme" @change="applyTheme">
      <option v-for="theme in themes" :key="theme.name" :value="theme.name">
        {{ theme.name }} ({{ theme.dark ? '暗色' : '亮色' }})
      </option>
    </select>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Terminal } from '@xterm/xterm';
import { getTerminalThemes, type TerminalTheme } from '@/api/terminal';

const themes = ref<TerminalTheme[]>([]);
const selectedTheme = ref<string>('Catppuccin Mocha');
const terminal = ref<Terminal | null>(null);

onMounted(async () => {
  // 加载主题列表
  themes.value = await getTerminalThemes();
  
  // 初始化终端
  terminal.value = new Terminal({
    fontFamily: 'Cascadia Mono, Consolas, monospace',
    fontSize: 14,
  });
  
  // 应用默认主题
  applyTheme();
});

function applyTheme() {
  const theme = themes.value.find(t => t.name === selectedTheme.value);
  if (theme && terminal.value) {
    terminal.value.options.theme = theme.schema;
  }
}
</script>
```

## 主题预览

### Catppuccin Mocha (推荐暗色主题)

```
背景: #1E1E2E (深蓝灰色)
前景: #CDD6F4 (浅蓝白色)
特点: 温暖、舒适、护眼
适用: 长时间编码、夜间使用
```

### Catppuccin Latte (推荐亮色主题)

```
背景: #EFF1F5 (浅灰白色)
前景: #4C4F69 (深蓝灰色)
特点: 柔和、清新、不刺眼
适用: 白天使用、演示场景
```

### GitHub Dark

```
背景: #101216 (深黑色)
前景: #8B949E (中灰色)
特点: 高对比度、专业感
适用: 熟悉 GitHub 界面的用户
```

## 常见问题

### Q: 如何添加自定义主题？

A: 有两种方式：

1. **通过 SQL 插入**（推荐用于批量添加）：
   ```sql
   INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
   SELECT 
       dk.id,
       'My Theme',
       '{"background":"#000000","foreground":"#FFFFFF",...}',
       'My Theme',
       '{"dark": true}',
       1000,
       'admin',
       'admin',
       NOW(),
       NOW(),
       0
   FROM sys_dict_key dk
   WHERE dk.key_name = 'terminalTheme';
   ```

2. **通过管理界面**（开发中）：
   - 系统设置 → 字典管理 → terminalTheme → 添加

### Q: 主题不生效怎么办？

A: 检查以下几点：

1. 确认 API 返回了主题数据
2. 确认前端正确解析了 JSON
3. 确认 xterm.js 版本兼容（推荐 5.x+）
4. 检查浏览器控制台是否有错误

### Q: 如何导出/导入主题？

A: 使用 SQL 导出：

```bash
# 导出所有主题
psql -U vail -d vail -c "
  SELECT jsonb_build_object(
    'name', label,
    'dark', (extra::jsonb->>'dark')::boolean,
    'schema', value::jsonb
  )
  FROM sys_dict_value dv
  JOIN sys_dict_key dk ON dv.key_id = dk.id
  WHERE dk.key_name = 'terminalTheme'
    AND dv.deleted = 0
  ORDER BY sort
" -t -A > themes.json
```

### Q: 支持哪些颜色格式？

A: 支持以下格式：
- 十六进制: `#1E1E2E`
- RGB: `rgb(30, 30, 46)`
- RGBA: `rgba(30, 30, 46, 0.9)`

推荐使用十六进制格式以保持一致性。

## 性能优化建议

1. **缓存主题列表**：主题数据不常变化，可以在前端缓存
2. **懒加载**：只在需要时加载主题列表
3. **预加载默认主题**：将默认主题内联到前端代码中

```typescript
// 默认主题（避免首次加载闪烁）
const DEFAULT_THEME = {
  name: 'Catppuccin Mocha',
  dark: true,
  schema: {
    background: '#1E1E2E',
    foreground: '#CDD6F4',
    // ... 其他颜色
  }
};

// 异步加载完整主题列表
const themes = ref<TerminalTheme[]>([DEFAULT_THEME]);
getTerminalThemes().then(data => {
  themes.value = data;
});
```

## 下一步

- 查看 [完整文档](./terminal-themes.md)
- 了解 [API 规范](../vail-rs/src/api/terminal.rs)
- 参考 [Orion Visor 前端实现](../orion-visor/orion-visor-ui/src/views/terminal/)
