#[cfg(target_os = "windows")]
pub use smtc_real::*;

#[cfg(not(target_os = "windows"))]
mod mock {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct NowPlayingInfo {
        pub title: Option<String>,
        pub artist: Option<String>,
        pub album: Option<String>,
        pub album_artist: Option<String>,
        pub cover_url: Option<String>,
        pub duration_ms: u64,
        pub progress_ms: u64,
        pub status: PlaybackStatus,
        pub player_name: String,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub enum PlaybackStatus {
        #[default]
        Stopped,
        Playing,
        Paused,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct SmtcSessionInfo {
        pub source_app_user_model_id: String,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub enum MediaCommand {
        #[default]
        None,
        Play,
        Pause,
        Next,
        Previous,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TextConversionMode {
        #[default]
        None,
        Simplified,
        Traditional,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub enum MediaUpdate {
        #[default]
        None,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub enum RepeatMode {
        #[default]
        None,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct SmtcControlCommand;
}

#[cfg(not(target_os = "windows"))]
pub use mock::*;
