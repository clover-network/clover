/// AssetId payment weight info getter
pub trait AssetIdWeightGetter<AssetId> {
  /// get the units per second that the asset_id has to pay
  fn get_units_per_second(asset_id: AssetId) -> Option<u128>;
}
