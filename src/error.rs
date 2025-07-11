#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Phidget Error: {0}")]
    Phidget(#[from] phidget::Error),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Serde Json Error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Menu Error: {0}")]
    Menu(#[from] menu::error::Error),
    #[error("Failed to start scale")]
    Initialization,
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("USB Error: {0}")]
    Rusb(#[from] rusb::Error),
    #[error("Couldn't Cast String to Int")]
    ParseInt,
}
