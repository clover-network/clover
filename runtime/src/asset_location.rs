use codec::{Decode, Encode};
use sp_std::prelude::*;
use xcm::v1::MultiLocation;

/// abstraction of the asset location
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, scale_info::TypeInfo)]
pub enum AssetLocation {
  /// Note: The current location is extracted as a special value
  /// Other locationing method might also have a current location
  /// e.g. MultiLocation::here()
  /// Which should be normalized to Current
  Current, // Current
  Para(MultiLocation),
}

impl Default for AssetLocation {
  fn default() -> Self {
    Self::Current
  }
}

impl AssetLocation {
  fn normalize(self) -> Self {
    match self {
      Self::Current => self,
      Self::Para(l) if l == MultiLocation::here() => Self::Current,
      _ => self,
    }
  }
}

impl From<MultiLocation> for AssetLocation {
  fn from(f: MultiLocation) -> Self {
    Self::Para(f).normalize()
  }
}

impl Into<Option<MultiLocation>> for AssetLocation {
  fn into(self) -> Option<MultiLocation> {
    match self {
      Self::Current => Some(MultiLocation::here()),
      Self::Para(l) => Some(l),
    }
  }
}
