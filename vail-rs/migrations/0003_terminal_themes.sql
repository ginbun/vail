-- 0003_terminal_themes.sql
-- Add terminal theme support

-- Insert terminal theme dictionary key
INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES ('terminalTheme', 'STRING', '[{"name": "dark", "type": "BOOLEAN"}]', 'Terminal theme', 'system', 'system', NOW(), NOW());

-- Insert terminal themes
-- Get the key_id for terminalTheme
DO $$
DECLARE
    theme_key_id BIGINT;
BEGIN
    SELECT id INTO theme_key_id FROM sys_dict_key WHERE key_name = 'terminalTheme';

    -- Dracula (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Dracula',
        '{"background":"#1E1F29","foreground":"#F8F8F2","cursor":"#BBBBBB","selectionBackground":"#44475A","black":"#000000","red":"#FF5555","green":"#50FA7B","yellow":"#F1FA8C","blue":"#BD93F9","cyan":"#8BE9FD","white":"#BBBBBB","brightBlack":"#555555","brightRed":"#FF5555","brightGreen":"#50FA7B","brightYellow":"#F1FA8C","brightBlue":"#BD93F9","brightCyan":"#8BE9FD","brightWhite":"#FFFFFF"}',
        'Dracula',
        '{"dark": true}',
        10,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Builtin Tango Light
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Builtin Tango Light',
        '{"background":"#FFFFFF","foreground":"#000000","cursor":"#000000","selectionBackground":"#B5D5FF","black":"#000000","red":"#CC0000","green":"#4E9A06","yellow":"#C4A000","blue":"#3465A4","cyan":"#06989A","white":"#D3D7CF","brightBlack":"#555753","brightRed":"#EF2929","brightGreen":"#8AE234","brightYellow":"#FCE94F","brightBlue":"#729FCF","brightCyan":"#34E2E2","brightWhite":"#EEEEEC"}',
        'Builtin Tango Light',
        '{"dark": false}',
        20,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Atom (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Atom',
        '{"background":"#161719","foreground":"#C5C8C6","cursor":"#D0D0D0","selectionBackground":"#444444","black":"#000000","red":"#FD5FF1","green":"#87C38A","yellow":"#FFD7B1","blue":"#85BEFD","cyan":"#85BEFD","white":"#E0E0E0","brightBlack":"#000000","brightRed":"#FD5FF1","brightGreen":"#94FA36","brightYellow":"#F5FFA8","brightBlue":"#96CBFE","brightCyan":"#85BEFD","brightWhite":"#E0E0E0"}',
        'Atom',
        '{"dark": true}',
        30,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- AtomOneLight
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'AtomOneLight',
        '{"background":"#F9F9F9","foreground":"#2A2C33","cursor":"#BBBBBB","selectionBackground":"#EDEDED","black":"#000000","red":"#DE3E35","green":"#3F953A","yellow":"#D2B67C","blue":"#2F5AF3","cyan":"#3F953A","white":"#BBBBBB","brightBlack":"#000000","brightRed":"#DE3E35","brightGreen":"#3F953A","brightYellow":"#D2B67C","brightBlue":"#2F5AF3","brightCyan":"#3F953A","brightWhite":"#FFFFFF"}',
        'AtomOneLight',
        '{"dark": false}',
        40,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- OneHalfDark
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'OneHalfDark',
        '{"background":"#282C34","foreground":"#DCDFE4","cursor":"#A3B3CC","selectionBackground":"#474E5D","black":"#282C34","red":"#E06C75","green":"#98C379","yellow":"#E5C07B","blue":"#61AFEF","cyan":"#56B6C2","white":"#DCDFE4","brightBlack":"#282C34","brightRed":"#E06C75","brightGreen":"#98C379","brightYellow":"#E5C07B","brightBlue":"#61AFEF","brightCyan":"#56B6C2","brightWhite":"#DCDFE4"}',
        'OneHalfDark',
        '{"dark": true}',
        50,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- OneHalfLight
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'OneHalfLight',
        '{"background":"#FAFAFA","foreground":"#383A42","cursor":"#BFCEFF","selectionBackground":"#BFCEFF","black":"#383A42","red":"#E45649","green":"#50A14F","yellow":"#C18401","blue":"#0184BC","cyan":"#0997B3","white":"#FAFAFA","brightBlack":"#4F525E","brightRed":"#E06C75","brightGreen":"#98C379","brightYellow":"#E5C07B","brightBlue":"#61AFEF","brightCyan":"#56B6C2","brightWhite":"#FFFFFF"}',
        'OneHalfLight',
        '{"dark": false}',
        60,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Apple System Colors (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Apple System Colors',
        '{"background":"#1E1E1E","foreground":"#FFFFFF","cursor":"#98989D","selectionBackground":"#3F638B","black":"#1A1A1A","red":"#CC372E","green":"#26A439","yellow":"#CDAC08","blue":"#0869CB","cyan":"#479EC2","white":"#98989D","brightBlack":"#464646","brightRed":"#FF453A","brightGreen":"#32D74B","brightYellow":"#FFD60A","brightBlue":"#0A84FF","brightCyan":"#76D6FF","brightWhite":"#FFFFFF"}',
        'Apple System Colors',
        '{"dark": true}',
        70,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Tomorrow (light)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Tomorrow',
        '{"background":"#FFFFFF","foreground":"#4D4D4C","cursor":"#4D4D4C","selectionBackground":"#D6D6D6","black":"#000000","red":"#C82829","green":"#718C00","yellow":"#EAB700","blue":"#4271AE","cyan":"#3E999F","white":"#FFFFFF","brightBlack":"#000000","brightRed":"#C82829","brightGreen":"#718C00","brightYellow":"#EAB700","brightBlue":"#4271AE","brightCyan":"#3E999F","brightWhite":"#FFFFFF"}',
        'Tomorrow',
        '{"dark": false}',
        80,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Catppuccin Mocha (dark) - 最流行的暗色主题
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Catppuccin Mocha',
        '{"background":"#1E1E2E","foreground":"#CDD6F4","cursor":"#F5E0DC","selectionBackground":"#585B70","black":"#45475A","red":"#F38BA8","green":"#A6E3A1","yellow":"#F9E2AF","blue":"#89B4FA","cyan":"#94E2D5","white":"#BAC2DE","brightBlack":"#585B70","brightRed":"#F38BA8","brightGreen":"#A6E3A1","brightYellow":"#F9E2AF","brightBlue":"#89B4FA","brightCyan":"#94E2D5","brightWhite":"#A6ADC8"}',
        'Catppuccin Mocha',
        '{"dark": true}',
        90,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Catppuccin Latte (light) - 最流行的亮色主题
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Catppuccin Latte',
        '{"background":"#EFF1F5","foreground":"#4C4F69","cursor":"#DC8A78","selectionBackground":"#ACB0BE","black":"#5C5F77","red":"#D20F39","green":"#40A02B","yellow":"#DF8E1D","blue":"#1E66F5","cyan":"#179299","white":"#ACB0BE","brightBlack":"#6C6F85","brightRed":"#D20F39","brightGreen":"#40A02B","brightYellow":"#DF8E1D","brightBlue":"#1E66F5","brightCyan":"#179299","brightWhite":"#BCC0CC"}',
        'Catppuccin Latte',
        '{"dark": false}',
        100,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Catppuccin Macchiato (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Catppuccin Macchiato',
        '{"background":"#24273A","foreground":"#CAD3F5","cursor":"#F4DBD6","selectionBackground":"#5B6078","black":"#494D64","red":"#ED8796","green":"#A6DA95","yellow":"#EED49F","blue":"#8AADF4","cyan":"#8BD5CA","white":"#B8C0E0","brightBlack":"#5B6078","brightRed":"#ED8796","brightGreen":"#A6DA95","brightYellow":"#EED49F","brightBlue":"#8AADF4","brightCyan":"#8BD5CA","brightWhite":"#A5ADCB"}',
        'Catppuccin Macchiato',
        '{"dark": true}',
        110,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Catppuccin Frappe (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Catppuccin Frappe',
        '{"background":"#303446","foreground":"#C6D0F5","cursor":"#F2D5CF","selectionBackground":"#626880","black":"#51576D","red":"#E78284","green":"#A6D189","yellow":"#E5C890","blue":"#8CAAEE","cyan":"#81C8BE","white":"#B5BFE2","brightBlack":"#626880","brightRed":"#E78284","brightGreen":"#A6D189","brightYellow":"#E5C890","brightBlue":"#8CAAEE","brightCyan":"#81C8BE","brightWhite":"#A5ADCE"}',
        'Catppuccin Frappe',
        '{"dark": true}',
        130,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- BlulocoLight
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'BlulocoLight',
        '{"background":"#F9F9F9","foreground":"#373A41","cursor":"#F32759","selectionBackground":"#DAF0FF","black":"#373A41","red":"#D52753","green":"#23974A","yellow":"#DF631C","blue":"#275FE4","cyan":"#27618D","white":"#BABBC2","brightBlack":"#676A77","brightRed":"#FF6480","brightGreen":"#3CBC66","brightYellow":"#C5A332","brightBlue":"#0099E1","brightCyan":"#6D93BB","brightWhite":"#D3D3D3"}',
        'BlulocoLight',
        '{"dark": false}',
        120,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- MaterialDesignColors (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'MaterialDesignColors',
        '{"background":"#1D262A","foreground":"#E7EBED","cursor":"#EAEAEA","selectionBackground":"#4E6A78","black":"#435B67","red":"#FC3841","green":"#5CF19E","yellow":"#FED032","blue":"#37B6FF","cyan":"#59FFD1","white":"#FFFFFF","brightBlack":"#A1B0B8","brightRed":"#FC746D","brightGreen":"#ADF7BE","brightYellow":"#FEE16C","brightBlue":"#70CFFF","brightCyan":"#9AFFE6","brightWhite":"#FFFFFF"}',
        'MaterialDesignColors',
        '{"dark": true}',
        140,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- GitHub Dark
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'GitHub Dark',
        '{"background":"#101216","foreground":"#8B949E","cursor":"#C9D1D9","selectionBackground":"#3B5070","black":"#000000","red":"#F78166","green":"#56D364","yellow":"#E3B341","blue":"#6CA4F8","cyan":"#2B7489","white":"#FFFFFF","brightBlack":"#4D4D4D","brightRed":"#F78166","brightGreen":"#56D364","brightYellow":"#E3B341","brightBlue":"#6CA4F8","brightCyan":"#2B7489","brightWhite":"#FFFFFF"}',
        'GitHub Dark',
        '{"dark": true}',
        150,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- GitHub Light
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'GitHub Light',
        '{"background":"#F4F4F4","foreground":"#3E3E3E","cursor":"#3F3F3F","selectionBackground":"#A9C1E2","black":"#3E3E3E","red":"#970B16","green":"#07962A","yellow":"#F8EEC7","blue":"#003E8A","cyan":"#89D1EC","white":"#FFFFFF","brightBlack":"#666666","brightRed":"#DE0000","brightGreen":"#87D5A2","brightYellow":"#F1D007","brightBlue":"#2E6CBA","brightCyan":"#1CFAFE","brightWhite":"#FFFFFF"}',
        'GitHub Light',
        '{"dark": false}',
        160,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- DimmedMonokai (dark)
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'DimmedMonokai',
        '{"background":"#1F1F1F","foreground":"#B9BCBA","cursor":"#F83E19","selectionBackground":"#2A2D32","black":"#3A3D43","red":"#BE3F48","green":"#879A3B","yellow":"#C5A635","blue":"#4F76A1","cyan":"#578FA4","white":"#B9BCBA","brightBlack":"#888987","brightRed":"#FB001F","brightGreen":"#0F722F","brightYellow":"#C47033","brightBlue":"#186DE3","brightCyan":"#2E706D","brightWhite":"#FDFFB9"}',
        'DimmedMonokai',
        '{"dark": true}',
        170,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Duotone Dark
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (
        theme_key_id,
        'Duotone Dark',
        '{"background":"#1F1D27","foreground":"#B7A1FF","cursor":"#FF9839","selectionBackground":"#353147","black":"#1F1D27","red":"#D9393E","green":"#2DCD73","yellow":"#D9B76E","blue":"#FFC284","cyan":"#2488FF","white":"#B7A1FF","brightBlack":"#353147","brightRed":"#D9393E","brightGreen":"#2DCD73","brightYellow":"#D9B76E","brightBlue":"#FFC284","brightCyan":"#2488FF","brightWhite":"#EAE5FF"}',
        'Duotone Dark',
        '{"dark": true}',
        180,
        'system',
        'system',
        NOW(),
        NOW(),
        0
    );

    -- Create history records for all themes
    INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
    SELECT id, '', value, NOW()
    FROM sys_dict_value
    WHERE key_id = theme_key_id;

END $$;
