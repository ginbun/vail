import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const storeSource = readFileSync(new URL('../src/store/modules/terminal/index.ts', import.meta.url), 'utf8');
const sshInteractBlockSource = readFileSync(
  new URL('../src/views/terminal/components/setting/general/terminal-ssh-interact-block.vue', import.meta.url),
  'utf8'
);

test('ssh interact preference updates local state so changes take effect immediately', () => {
  assert.match(
    sshInteractBlockSource,
    /useTerminalPreference<TerminalSshInteractSetting>\(TerminalPreferenceItem\.SSH_INTERACT_SETTING,\s*true/,
  );
});

test('right click paste is disabled by default', () => {
  assert.match(storeSource, /rightClickPaste:\s*false/);
});
