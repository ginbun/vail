use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Default)]
pub struct SshCommandAuditSnapshot {
    current_line: String,
    commands: Vec<String>,
    truncated: bool,
}

impl SshCommandAuditSnapshot {
    const MAX_COMMANDS: usize = 2000;

    pub fn ingest(&mut self, chunk: &str) {
        for ch in chunk.chars() {
            match ch {
                '\r' | '\n' => self.flush_current_line(),
                '\u{0008}' | '\u{007f}' => {
                    self.current_line.pop();
                }
                _ => self.current_line.push(ch),
            }
        }
    }

    pub fn finish_line(&mut self) {
        self.flush_current_line();
    }

    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn into_commands(self) -> Vec<String> {
        self.commands
    }

    fn flush_current_line(&mut self) {
        let line = self.current_line.trim();
        if !line.is_empty() {
            if self.commands.len() < Self::MAX_COMMANDS {
                self.commands.push(mask_command_for_audit(line));
            } else {
                self.truncated = true;
            }
        }
        self.current_line.clear();
    }
}

static KV_SECRET_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        "password",
        "passwd",
        "pwd",
        "token",
        "secret",
        "access_key",
        "apikey",
    ]
    .iter()
    .filter_map(|key| Regex::new(&format!(r"(?i)({key}\s*[=:]\s*)(\S+)")).ok())
    .collect()
});

static AUTH_HEADER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(authorization\s*:\s*)(\S+)").expect("valid auth regex")
});

static MYSQL_INLINE_PASSWORD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(mysql\s+.*?\s-p)(\S+)").expect("valid mysql regex")
});

pub fn mask_command_for_audit(command: &str) -> String {
    let mut masked = command.to_string();
    for re in KV_SECRET_PATTERNS.iter() {
        masked = re.replace_all(&masked, "${1}***").to_string();
    }

    masked = AUTH_HEADER_PATTERN
        .replace_all(&masked, "${1}***")
        .to_string();

    MYSQL_INLINE_PASSWORD_PATTERN
        .replace_all(&masked, "${1}***")
        .to_string()
}
