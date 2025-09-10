use std::fmt;

use crate::app_definition::AppView;
use crate::app_settings::AppSettings;
use crate::error::AppResult;
use crate::types::LrcContentType;
use egui_toast::Toast;
use lyrics_helper_core::FullConversionResult;
use lyrics_helper_core::LyricFormat;
use lyrics_helper_core::LyricsAndMetadata;
use lyrics_helper_core::SearchResult;
use lyrics_helper_core::model::track::FullLyricsResult;

// 主事件枚举
#[derive(Debug, Clone)]
pub enum UserAction {
    File(FileAction),
    Lyrics(Box<LyricsAction>),
    Player(PlayerAction),
    UI(UIAction),
    Settings(SettingsAction),
    AmllConnector(AmllConnectorAction),
    Downloader(Box<DownloaderAction>),
}

// 子事件枚举定义
#[derive(Debug, Clone)]
pub enum FileAction {
    Open,
    Save,
    LoadTranslationLrc,
    LoadRomanizationLrc,
}

#[derive(Debug, Clone)]
pub enum LyricsAction {
    Convert,
    ConvertCompleted(AppResult<FullConversionResult>),
    ConvertChinese(ferrous_opencc::config::BuiltinConfig),
    SourceFormatChanged(LyricFormat),
    TargetFormatChanged(LyricFormat),
    AddMetadata,                             // 添加新的元数据条目
    DeleteMetadata(usize),                   // 删除指定索引的元数据条目
    UpdateMetadataKey(usize, String),        // 更新指定索引的元数据键
    UpdateMetadataValue(usize, String),      // 更新指定索引的元数据值
    ToggleMetadataPinned(usize),             // 切换指定索引的元数据固定状态
    LrcInputChanged(String, LrcContentType), // 当LRC文本框内容改变时
    MainInputChanged(String),                // 当主输入文本框内容改变时
    ClearAllData,
    LoadFetchedResult(FullLyricsResult),
    ApplyFetchedLyrics(Box<LyricsAndMetadata>), // 应用获取到的歌词
    LoadFileContent(String, std::path::PathBuf),
    ApplyProcessor(ProcessorType),
}

#[derive(Debug, Clone)]
pub enum DownloaderAction {
    SetTitle(String),
    SetArtist(String),
    FillFromSmtc,
    PerformSearch,
    SearchCompleted(AppResult<Vec<SearchResult>>),
    SelectResultForPreview(SearchResult),
    PreviewDownloadCompleted(AppResult<FullLyricsResult>),
    ApplyAndClose,
    Close,
}

#[derive(Debug, Clone)]
pub enum PlayerAction {
    /// 让 smtc-suite 选择一个新的媒体会话。
    SelectSmtcSession(String),
    /// 设置时间轴偏移量
    SetSmtcTimeOffset(i64),
    /// 保存当前歌词到本地缓存。
    SaveToLocalCache,
    /// 更新封面数据。
    UpdateCover(Option<Vec<u8>>),
    /// 控制 smtc-suite 的音频捕获功能
    ToggleAudioCapture(bool),
}

#[derive(Debug, Clone)]
pub enum PanelType {
    Settings,
    Metadata,
    Log,
    Markers,
    Translation,
    Romanization,
    AmllConnector,
}

#[derive(Clone)]
pub enum UIAction {
    SetPanelVisibility(PanelType, bool),
    SetView(AppView),
    SetWrapText(bool),
    ShowPanel(PanelType),
    HidePanel(PanelType),
    ClearLogs,
    StopOtherSearches,
    ShowToast(Box<Toast>),
}

impl fmt::Debug for UIAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetPanelVisibility(panel, is_visible) => f
                .debug_tuple("SetPanelVisibility")
                .field(panel)
                .field(is_visible)
                .finish(),
            Self::SetView(view) => f.debug_tuple("SetView").field(view).finish(),
            Self::SetWrapText(wrap) => f.debug_tuple("SetWrapText").field(wrap).finish(),
            Self::ShowPanel(panel) => f.debug_tuple("ShowPanel").field(panel).finish(),
            Self::HidePanel(panel) => f.debug_tuple("HidePanel").field(panel).finish(),
            Self::ClearLogs => write!(f, "ClearLogs"),
            Self::StopOtherSearches => write!(f, "StopOtherSearches"),
            Self::ShowToast(_) => f.debug_tuple("ShowToast").field(&"<Box<Toast>>").finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingsAction {
    Save(Box<AppSettings>),
    Cancel,
    Reset,
}

#[derive(Debug, Clone)]
pub enum AmllConnectorAction {
    Connect,
    Disconnect,
    Retry,
    CheckIndexUpdate,
    ReloadProviders,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    MetadataStripper,
    SyllableSmoother,
    AgentRecognizer,
}
