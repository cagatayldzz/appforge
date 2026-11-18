use crate::Error;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

/// Apple accepts tokens that live at most 20 minutes.
const TOKEN_LIFETIME_SECONDS: u64 = 15 * 60;
const AUDIENCE: &str = "appstoreconnect-v1";

#[derive(Clone)]
pub struct Credentials {
  pub issuer_id: String,
  pub key_id: String,
  /// The contents of the `.p8` file (PKCS #8 PEM).
  pub private_key: String,
}

#[derive(Serialize)]
struct Claims<'a> {
  iss: &'a str,
  iat: u64,
  exp: u64,
  aud: &'a str,
}

impl Credentials {
  /// A short-lived ES256 token for the `Authorization: Bearer` header. `now` is seconds since the Unix epoch.
  pub fn token(&self, now: u64) -> Result<String, Error> {
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(self.key_id.trim().to_owned());
    let claims = Claims { iss: self.issuer_id.trim(), iat: now, exp: now + TOKEN_LIFETIME_SECONDS, aud: AUDIENCE };
    let key = EncodingKey::from_ec_pem(self.private_key.as_bytes()).map_err(|error| Error::InvalidKey(error.to_string()))?;
    encode(&header, &claims, &key).map_err(|error| Error::InvalidKey(error.to_string()))
  }
}

/// Checks that `pem` can sign tokens, without any network access.
pub fn validate_private_key(pem: &str) -> Result<(), Error> {
  EncodingKey::from_ec_pem(pem.as_bytes()).map(|_| ()).map_err(|_| Error::InvalidKey("expected the contents of an App Store Connect .p8 file".into()))
}
