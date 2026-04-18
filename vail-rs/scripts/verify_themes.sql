-- 验证终端主题数据
-- 使用方法: psql -U vail -d vail -f scripts/verify_themes.sql

\echo '🎨 Terminal Themes Verification'
\echo '================================'
\echo ''

-- 检查字典键是否存在
\echo '📋 Checking dictionary key...'
SELECT 
    id,
    key_name,
    value_type,
    description,
    create_time
FROM sys_dict_key 
WHERE key_name = 'terminalTheme';

\echo ''
\echo '📊 Theme statistics:'
SELECT 
    COUNT(*) as total_themes,
    COUNT(*) FILTER (WHERE extra::jsonb->>'dark' = 'true') as dark_themes,
    COUNT(*) FILTER (WHERE extra::jsonb->>'dark' = 'false') as light_themes
FROM sys_dict_value dv
JOIN sys_dict_key dk ON dv.key_id = dk.id
WHERE dk.key_name = 'terminalTheme'
  AND dv.deleted = 0;

\echo ''
\echo '🐱 Catppuccin themes:'
SELECT 
    label as theme_name,
    extra::jsonb->>'dark' as is_dark,
    sort
FROM sys_dict_value dv
JOIN sys_dict_key dk ON dv.key_id = dk.id
WHERE dk.key_name = 'terminalTheme'
  AND dv.deleted = 0
  AND label LIKE '%Catppuccin%'
ORDER BY sort;

\echo ''
\echo '📝 All available themes:'
SELECT 
    sort,
    label as theme_name,
    CASE 
        WHEN extra::jsonb->>'dark' = 'true' THEN '🌙 Dark'
        ELSE '☀️  Light'
    END as type,
    LENGTH(value::text) as schema_size
FROM sys_dict_value dv
JOIN sys_dict_key dk ON dv.key_id = dk.id
WHERE dk.key_name = 'terminalTheme'
  AND dv.deleted = 0
ORDER BY sort;

\echo ''
\echo '🎨 Sample theme (Catppuccin Mocha):'
SELECT 
    label as theme_name,
    value::jsonb->>'background' as background,
    value::jsonb->>'foreground' as foreground,
    value::jsonb->>'cursor' as cursor
FROM sys_dict_value dv
JOIN sys_dict_key dk ON dv.key_id = dk.id
WHERE dk.key_name = 'terminalTheme'
  AND dv.deleted = 0
  AND label = 'Catppuccin Mocha';

\echo ''
\echo '✅ Verification complete!'
