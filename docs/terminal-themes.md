# 终端主题功能

## 概述

Vail 支持丰富的终端主题配置，包括流行的 Catppuccin 系列主题。所有主题数据存储在数据库的字典表中，通过 REST API 提供给前端。

## 已添加的主题

### Catppuccin 系列（推荐）

Catppuccin 是一个温暖的中对比度配色方案，有四个变体：

1. **Catppuccin Mocha** (暗色) - 最流行的暗色主题
   - 背景色: `#1E1E2E`
   - 前景色: `#CDD6F4`
   - 特点: 温暖、舒适的暗色调

2. **Catppuccin Latte** (亮色) - 最流行的亮色主题
   - 背景色: `#EFF1F5`
   - 前景色: `#4C4F69`
   - 特点: 柔和、护眼的亮色调

3. **Catppuccin Macchiato** (暗色)
   - 背景色: `#24273A`
   - 前景色: `#CAD3F5`
   - 特点: 介于 Mocha 和 Frappe 之间

4. **Catppuccin Frappe** (暗色)
   - 背景色: `#303446`
   - 前景色: `#C6D0F5`
   - 特点: 更深的暗色调

### 其他流行主题

- **Dracula** - 经典的暗色主题
- **Atom** / **AtomOneLight** - Atom 编辑器风格
- **OneHalfDark** / **OneHalfLight** - 平衡的对比度
- **GitHub Dark** / **GitHub Light** - GitHub 风格
- **Apple System Colors** - macOS 系统配色
- **Tomorrow** - 简洁的亮色主题
- **MaterialDesignColors** - Material Design 风格
- **DimmedMonokai** - 柔和的 Monokai
- **Duotone Dark** - 双色调暗色主题
- **BlulocoLight** - 蓝色调亮色主题
- **Builtin Tango Light** - 内置 Tango 亮色主题

## 数据库结构

### 字典键 (sys_dict_key)

```sql
key_name: 'terminalTheme'
value_type: 'STRING'
extra_schema: '[{"name": "dark", "type": "BOOLEAN"}]'
description: 'Terminal theme'
```

### 字典值 (sys_dict_value)

每个主题包含：
- `name`: 主题名称
- `value`: JSON 格式的主题配色方案
- `label`: 显示标签
- `extra`: 额外信息（如 `{"dark": true}`）
- `sort`: 排序顺序

### 主题配色方案结构

```json
{
  "background": "#1E1E2E",
  "foreground": "#CDD6F4",
  "cursor": "#F5E0DC",
  "selectionBackground": "#585B70",
  "black": "#45475A",
  "red": "#F38BA8",
  "green": "#A6E3A1",
  "yellow": "#F9E2AF",
  "blue": "#89B4FA",
  "cyan": "#94E2D5",
  "white": "#BAC2DE",
  "brightBlack": "#585B70",
  "brightRed": "#F38BA8",
  "brightGreen": "#A6E3A1",
  "brightYellow": "#F9E2AF",
  "brightBlue": "#89B4FA",
  "brightCyan": "#94E2D5",
  "brightWhite": "#A6ADC8"
}
```

## API 接口

### 获取终端主题列表

**请求:**
```http
GET /api/terminal/themes
```

**响应:**
```json
[
  {
    "name": "Catppuccin Mocha",
    "dark": true,
    "schema": {
      "background": "#1E1E2E",
      "foreground": "#CDD6F4",
      ...
    }
  },
  {
    "name": "Catppuccin Latte",
    "dark": false,
    "schema": {
      "background": "#EFF1F5",
      "foreground": "#4C4F69",
      ...
    }
  }
]
```

## 迁移文件

主题数据通过数据库迁移文件初始化：

```
vail-rs/migrations/0003_terminal_themes.sql
```

该迁移文件会：
1. 创建 `terminalTheme` 字典键
2. 插入所有预定义的主题数据
3. 创建主题变更历史记录

## 使用方法

### 后端开发

1. 运行数据库迁移：
   ```bash
   cd vail-rs
   cargo run
   ```
   应用启动时会自动运行迁移脚本。

2. 测试 API：
   ```bash
   curl http://localhost:3000/api/terminal/themes
   ```

### 前端集成

前端可以通过 API 获取主题列表，然后应用到终端组件：

```typescript
// 获取主题列表
const response = await fetch('/api/terminal/themes');
const themes = await response.json();

// 应用主题到 xterm.js
import { Terminal } from '@xterm/xterm';

const terminal = new Terminal({
  theme: themes[0].schema  // 使用第一个主题
});
```

## 添加新主题

如果需要添加新主题，可以：

1. **通过 SQL 直接插入：**
   ```sql
   INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
   SELECT 
       dk.id,
       'My Custom Theme',
       '{"background":"#000000","foreground":"#FFFFFF",...}',
       'My Custom Theme',
       '{"dark": true}',
       200,
       'admin',
       'admin',
       NOW(),
       NOW(),
       0
   FROM sys_dict_key dk
   WHERE dk.key_name = 'terminalTheme';
   ```

2. **通过管理界面：**
   - 访问系统设置 → 字典管理
   - 找到 `terminalTheme` 键
   - 添加新的字典值

## 兼容性

- 主题配色方案与 xterm.js 完全兼容
- 与 Orion Visor 前端 API 契约保持一致
- 支持 PostgreSQL 18+ 数据库

## 参考资源

- [Catppuccin 官网](https://catppuccin.com/)
- [xterm.js 主题文档](https://xtermjs.org/docs/api/terminal/interfaces/itheme/)
- [Orion Visor 项目](https://github.com/dromara/orion-visor)

## 安全注意事项

根据 AGENTS.md 中的安全规则：

- ✅ 主题数据存储在数据库中，不包含敏感信息
- ✅ API 端点为只读操作，无需特殊权限
- ✅ 主题配色方案经过 JSON 验证，防止注入攻击
- ✅ 符合最小权限原则，普通用户可访问

## 故障排查

### 主题列表为空

1. 检查数据库迁移是否成功运行：
   ```sql
   SELECT * FROM sys_dict_key WHERE key_name = 'terminalTheme';
   SELECT COUNT(*) FROM sys_dict_value WHERE key_id = (
       SELECT id FROM sys_dict_key WHERE key_name = 'terminalTheme'
   );
   ```

2. 检查应用日志是否有错误信息

3. 手动运行迁移脚本：
   ```bash
   psql -U vail -d vail -f vail-rs/migrations/0003_terminal_themes.sql
   ```

### API 返回错误

1. 检查数据库连接是否正常
2. 验证 `sys_dict_value.value` 字段是否为有效 JSON
3. 查看应用日志中的详细错误信息

## 未来计划

- [ ] 支持用户自定义主题
- [ ] 主题预览功能
- [ ] 主题导入/导出
- [ ] 主题市场/社区分享
- [ ] 自动切换暗色/亮色主题（跟随系统）
