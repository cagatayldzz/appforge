use std::fmt;

#[derive(Debug)]
pub enum Error {
  /// The `.p8` file is not a usable private key.
  InvalidKey(String),
  Network(String),
  Api {
    status: u16,
    message: String,
  },
  Unexpected(String),
}

impl fmt::Display for Error {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidKey(message) => write!(formatter, "The private key is not valid: {message}"),
      Self::Network(message) => write!(formatter, "Could not reach App Store Connect: {message}"),
      Self::Api { status: 401, .. } => {
        write!(formatter, "Apple rejected the API key (401). Check the Issuer ID, the Key ID and that the .p8 file matches that key.")
      }
      Self::Api { status: 403, message } => write!(formatter, "This API key's role is not allowed to do that (403): {message}"),
      Self::Api { status, message } => write!(formatter, "App Store Connect returned an error ({status}): {message}"),
      Self::Unexpected(message) => write!(formatter, "Unexpected answer from App Store Connect: {message}"),
    }
  }
}

impl std::error::Error for Error {}
