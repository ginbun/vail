-- 0006_terminal_emulation_type.sql
-- Add terminal emulation type dictionary support

INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES ('terminalEmulationType', 'STRING', '[]', '终端仿真类型', 'system', 'system', NOW(), NOW());

DO $$
DECLARE
    emulation_type_key_id BIGINT;
BEGIN
    -- Get key ID
    SELECT id INTO emulation_type_key_id FROM sys_dict_key WHERE key_name = 'terminalEmulationType';

    -- vt100
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (emulation_type_key_id, 'vt100', 'vt100', 'vt100', '{}', 10, 'system', 'system', NOW(), NOW(), 0);

    -- xterm
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (emulation_type_key_id, 'xterm', 'xterm', 'xterm', '{}', 20, 'system', 'system', NOW(), NOW(), 0);

    -- xterm-16color
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (emulation_type_key_id, 'xterm-16color', 'xterm-16color', '16color', '{}', 30, 'system', 'system', NOW(), NOW(), 0);

    -- xterm-256color
    INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
    VALUES (emulation_type_key_id, 'xterm-256color', 'xterm-256color', '256color', '{}', 40, 'system', 'system', NOW(), NOW(), 0);

    -- Create history records
    INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
    SELECT id, '', value, NOW()
    FROM sys_dict_value
    WHERE key_id = emulation_type_key_id;

END $$;
