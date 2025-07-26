use thiserror::Error;

#[derive(Debug, Error)]
pub enum WoofError {
  #[error("Path is not a directory: {0}")]
  InvalidInputDirectory(String),

  #[error("Invalid file name: {0}, expected flat or namespaced format")]
  InvalidFileName(String),

  #[error("Found both flat and namespaced files")]
  MixedFileModes,

  #[error("Io error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Error parsing translation file {0}: {1}")]
  Toml(String, toml::de::Error),

  #[error("File exists at output path {0}")]
  OutputFileExists(String),
}
