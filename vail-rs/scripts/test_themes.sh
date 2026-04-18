#!/bin/bash
# 测试终端主题 API

set -e

BASE_URL="${BASE_URL:-http://localhost:3000}"

echo "🎨 Testing Terminal Themes API"
echo "================================"
echo ""

# 测试获取主题列表
echo "📋 Fetching terminal themes..."
response=$(curl -s "${BASE_URL}/api/terminal/themes")

# 检查响应是否为有效 JSON
if ! echo "$response" | jq . > /dev/null 2>&1; then
    echo "❌ Invalid JSON response"
    echo "$response"
    exit 1
fi

# 统计主题数量
theme_count=$(echo "$response" | jq 'length')
echo "✅ Found $theme_count themes"
echo ""

# 显示所有主题名称
echo "📝 Available themes:"
echo "$response" | jq -r '.[] | "  - \(.name) (\(if .dark then "dark" else "light" end))"'
echo ""

# 检查 Catppuccin 主题
echo "🐱 Checking Catppuccin themes..."
catppuccin_count=$(echo "$response" | jq '[.[] | select(.name | contains("Catppuccin"))] | length')
echo "✅ Found $catppuccin_count Catppuccin variants"

catppuccin_themes=$(echo "$response" | jq -r '.[] | select(.name | contains("Catppuccin")) | "  - \(.name)"')
echo "$catppuccin_themes"
echo ""

# 显示第一个主题的详细信息
echo "🎨 Sample theme (first one):"
echo "$response" | jq '.[0]'
echo ""

# 验证主题结构
echo "🔍 Validating theme structure..."
has_required_fields=$(echo "$response" | jq '.[0] | has("name") and has("dark") and has("schema")')
if [ "$has_required_fields" = "true" ]; then
    echo "✅ Theme structure is valid"
else
    echo "❌ Theme structure is invalid"
    exit 1
fi

# 验证配色方案
echo "🎨 Validating color schema..."
has_colors=$(echo "$response" | jq '.[0].schema | has("background") and has("foreground") and has("cursor")')
if [ "$has_colors" = "true" ]; then
    echo "✅ Color schema is valid"
else
    echo "❌ Color schema is invalid"
    exit 1
fi

echo ""
echo "✅ All tests passed!"
