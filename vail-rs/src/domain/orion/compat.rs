#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrionCompatModule {
    ExecTemplate,
    ExecJob,
    ExecCommandLog,
    ExecJobLog,
    UploadTask,
    CommandSnippetGroup,
    CommandSnippet,
    PathBookmarkGroup,
    PathBookmark,
    TerminalConnectLog,
    TerminalFileLog,
    NotifyTemplate,
    Tag,
    HistoryValue,
    Favorite,
    Preference,
    SystemSetting,
}

impl OrionCompatModule {
    pub fn from_exec(value: &str) -> Option<Self> {
        match value {
            "exec-template" => Some(Self::ExecTemplate),
            "exec-job" => Some(Self::ExecJob),
            "exec-command-log" => Some(Self::ExecCommandLog),
            "exec-job-log" => Some(Self::ExecJobLog),
            "upload-task" => Some(Self::UploadTask),
            _ => None,
        }
    }

    pub fn from_terminal(value: &str) -> Option<Self> {
        match value {
            "command-snippet-group" => Some(Self::CommandSnippetGroup),
            "command-snippet" => Some(Self::CommandSnippet),
            "path-bookmark-group" => Some(Self::PathBookmarkGroup),
            "path-bookmark" => Some(Self::PathBookmark),
            "session" => Some(Self::TerminalConnectLog),
            "connect-log" => Some(Self::TerminalConnectLog),
            "file-log" => Some(Self::TerminalFileLog),
            _ => None,
        }
    }

    pub fn from_infra(value: &str) -> Option<Self> {
        match value {
            "notify-template" => Some(Self::NotifyTemplate),
            "tag" => Some(Self::Tag),
            "history-value" => Some(Self::HistoryValue),
            "favorite" => Some(Self::Favorite),
            "preference" => Some(Self::Preference),
            "system-setting" => Some(Self::SystemSetting),
            _ => None,
        }
    }

    pub fn store_key(self) -> &'static str {
        match self {
            Self::ExecTemplate => "orion:exec-template",
            Self::ExecJob => "orion:exec-job",
            Self::ExecCommandLog => "orion:exec-command-log",
            Self::ExecJobLog => "orion:exec-job-log",
            Self::UploadTask => "orion:upload-task",
            Self::CommandSnippetGroup => "orion:command-snippet-group",
            Self::CommandSnippet => "orion:command-snippet",
            Self::PathBookmarkGroup => "orion:path-bookmark-group",
            Self::PathBookmark => "orion:path-bookmark",
            Self::TerminalConnectLog => "orion:terminal-connect-log",
            Self::TerminalFileLog => "orion:terminal-file-log",
            Self::NotifyTemplate => "orion:notify-template",
            Self::Tag => "orion:tag",
            Self::HistoryValue => "orion:history-value",
            Self::Favorite => "orion:favorite",
            Self::Preference => "orion:preference",
            Self::SystemSetting => "orion:system-setting",
        }
    }
}
