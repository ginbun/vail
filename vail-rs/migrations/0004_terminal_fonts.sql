-- 0004_terminal_fonts.sql
-- Add terminal font family, size, and weight support

-- Insert terminal font family dictionary key
INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES ('terminalFontFamily', 'STRING', '[]', '终端字体样式', 'system', 'system', NOW(), NOW());

-- Insert terminal font size dictionary key
INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES ('terminalFontSize', 'INTEGER', '[]', '终端字体大小', 'system', 'system', NOW(), NOW());

-- Insert terminal font weight dictionary key
INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES ('terminalFontWeight', 'STRING', '[]', '终端文本粗细', 'system', 'system', NOW(), NOW());

-- Insert font families
DO $$
DECLARE
    font_family_key_id BIGINT;
    font_size_key_id BIGINT;
    font_weight_key_id BIGINT;
BEGIN
    -- Get key IDs
    SELECT id INTO font_family_key_id FROM sys_dict_key WHERE key_name = 'terminalFontFamily';
    SELECT id INTO font_size_key_id FROM sys_dict_key WHERE key_name = 'terminalFontSize';
    SELECT id INTO font_weight_key_id FROM sys_dict_key WHERE key_name = 'terminalFontWeight';

    -- ========================================
    -- Font Families
    -- ========================================

    -- Default (system default)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, '_', '_', '默认', '{}', 10, 'system', 'system', NOW(), NOW(), 0);

    -- Courier New (classic)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Courier New', 'Courier New', 'Courier New', '{}', 20, 'system', 'system', NOW(), NOW(), 0);

    -- Lucida Console
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Lucida Console', 'Lucida Console', 'Lucida Console', '{}', 30, 'system', 'system', NOW(), NOW(), 0);

    -- Courier
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Courier', 'Courier', 'Courier', '{}', 40, 'system', 'system', NOW(), NOW(), 0);

    -- Consolas (Windows)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Consolas', 'Consolas', 'Consolas', '{}', 50, 'system', 'system', NOW(), NOW(), 0);

    -- Fira Code (with ligatures) ⭐
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Fira Code', 'Fira Code', 'Fira Code', '{}', 60, 'system', 'system', NOW(), NOW(), 0);

    -- JetBrains Mono ⭐
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'JetBrains Mono', 'JetBrains Mono', 'JetBrains Mono', '{}', 70, 'system', 'system', NOW(), NOW(), 0);

    -- Source Code Pro
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Source Code Pro', 'Source Code Pro', 'Source Code Pro', '{}', 80, 'system', 'system', NOW(), NOW(), 0);

    -- Cascadia Mono (Windows Terminal)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Cascadia Mono', 'Cascadia Mono', 'Cascadia Mono', '{}', 90, 'system', 'system', NOW(), NOW(), 0);

    -- Cascadia Code (with ligatures)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Cascadia Code', 'Cascadia Code', 'Cascadia Code', '{}', 95, 'system', 'system', NOW(), NOW(), 0);

    -- Monaco (macOS)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Monaco', 'Monaco', 'Monaco', '{}', 100, 'system', 'system', NOW(), NOW(), 0);

    -- Menlo (macOS)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Menlo', 'Menlo', 'Menlo', '{}', 110, 'system', 'system', NOW(), NOW(), 0);

    -- SF Mono (macOS)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'SF Mono', 'SF Mono', 'SF Mono', '{}', 120, 'system', 'system', NOW(), NOW(), 0);

    -- Hack
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Hack', 'Hack', 'Hack', '{}', 130, 'system', 'system', NOW(), NOW(), 0);

    -- DejaVu Sans Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'DejaVu Sans Mono', 'DejaVu Sans Mono', 'DejaVu Sans Mono', '{}', 140, 'system', 'system', NOW(), NOW(), 0);

    -- Ubuntu Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Ubuntu Mono', 'Ubuntu Mono', 'Ubuntu Mono', '{}', 150, 'system', 'system', NOW(), NOW(), 0);

    -- Roboto Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'Roboto Mono', 'Roboto Mono', 'Roboto Mono', '{}', 160, 'system', 'system', NOW(), NOW(), 0);

    -- IBM Plex Mono
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_family_key_id, 'IBM Plex Mono', 'IBM Plex Mono', 'IBM Plex Mono', '{}', 170, 'system', 'system', NOW(), NOW(), 0);

    -- ========================================
    -- Font Sizes (10px - 24px)
    -- ========================================

    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES 
        (font_size_key_id, '10', '10', '10px', '{}', 10, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '11', '11', '11px', '{}', 20, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '12', '12', '12px', '{}', 30, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '13', '13', '13px', '{}', 40, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '14', '14', '14px', '{}', 50, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '15', '15', '15px', '{}', 60, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '16', '16', '16px', '{}', 70, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '17', '17', '17px', '{}', 80, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '18', '18', '18px', '{}', 90, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '20', '20', '20px', '{}', 100, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '22', '22', '22px', '{}', 110, 'system', 'system', NOW(), NOW(), 0),
        (font_size_key_id, '24', '24', '24px', '{}', 120, 'system', 'system', NOW(), NOW(), 0);

    -- ========================================
    -- Font Weights
    -- ========================================

    -- Normal
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_weight_key_id, 'normal', 'normal', '正常', '{}', 10, 'system', 'system', NOW(), NOW(), 0);

    -- Bold
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (font_weight_key_id, 'bold', 'bold', '加粗', '{}', 20, 'system', 'system', NOW(), NOW(), 0);

    -- Numeric weights
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES 
        (font_weight_key_id, '100', '100', '极细 (100)', '{}', 30, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '200', '200', '特细 (200)', '{}', 40, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '300', '300', '细 (300)', '{}', 50, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '400', '400', '正常 (400)', '{}', 60, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '500', '500', '中等 (500)', '{}', 70, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '600', '600', '半粗 (600)', '{}', 80, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '700', '700', '粗 (700)', '{}', 90, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '800', '800', '特粗 (800)', '{}', 100, 'system', 'system', NOW(), NOW(), 0),
        (font_weight_key_id, '900', '900', '极粗 (900)', '{}', 110, 'system', 'system', NOW(), NOW(), 0);

    -- Create history records
    INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
    SELECT id, '', value, NOW()
    FROM sys_dict_value
    WHERE key_id IN (font_family_key_id, font_size_key_id, font_weight_key_id);

END $$;
