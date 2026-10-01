use serde::{Serialize, Serializer};

use crate::format::human_bytes;

/// One error type for all Tauri commands.
/// The `Display` text is a friendly message that the UI shows as-is.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("This model is not in the list any more. Refresh the list and try again.")]
    UnknownModel,

    #[error("Another model is downloading. Please wait until it finishes, or pause it.")]
    OtherDownloadRunning,

    #[error("Not enough free disk space. This model needs {} more, but only {} is free.", human_bytes(*.need), human_bytes(*.free))]
    NotEnoughDisk { need: u64, free: u64 },

    #[error("This model needs a login. Not supported yet.")]
    NeedsLogin,

    #[error("The model file was not found online. The model list may be out of date.")]
    FileNotFoundOnline,

    #[error("The download stopped because of a network problem. Click Resume to try again.")]
    Network(String),

    #[error("The downloaded file is damaged, so it was removed. Please download it again.")]
    DamagedFile,

    #[error("Could not save the file on this computer: {0}")]
    Disk(String),

    #[error("This model is not fully downloaded yet.")]
    ModelNotReady,

    #[error("The model engine is missing. Please reinstall KuttyLM.")]
    EngineMissing,

    #[error("Could not reach Hugging Face. Check your internet connection and try again.")]
    HuggingFaceUnavailable,

    #[error("This model is for document search. It cannot chat.")]
    NotAChatModel,

    #[error("Documents need the document search model. Download it first.")]
    EmbedModelMissing,

    #[error("{0}")]
    Document(String),

    #[error("{0}")]
    InvalidSetting(String),

    #[error("Port {0} is used by another program. Choose another port in Settings.")]
    PortBusy(u16),

    #[error("Something went wrong inside the app: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Disk(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
