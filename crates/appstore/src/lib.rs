//! A blocking client for the App Store Connect API (<https://developer.apple.com/documentation/appstoreconnectapi>).
//!
//! Sign-in is an API key (Issuer ID, Key ID and the `.p8` private key) created in App Store Connect under
//! Users and Access > Integrations; Apple offers no Apple ID password login for automation. Every call blocks,
//! so run it off the interface thread.

mod auth;
mod client;
mod error;
mod model;
mod territories;

pub use auth::{validate_private_key, Credentials};
pub use client::distribution::{Distribution, ReviewSubmission, StoreBuild, StoreVersion};

pub use client::pricing::{Availability, PricePeriod, PricePoint, Pricing, TerritoryAvailability, TerritoryPrice};
pub use client::product_page::ProductAsset;
pub use client::reviews::{CustomerReview, ReviewResponse};
pub use client::{Client, DEFAULT_BASE_URL};
pub use error::Error;
pub use model::{
  AgeRatingDeclaration, App, AppInfoLocalization, AppSnapshot, EncryptionDeclaration, GeneralInformation, ReviewDetail, ScreenshotSet, Version,
  VersionLocalization, AGE_FLAG_KEYS, AGE_LEVEL_KEYS,
};
pub use territories::{territory_code, territory_name};
