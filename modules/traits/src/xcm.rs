/// AssetId payment weight info getter
pub trait AssetIdWeightGetter<AssetId> {
    /// get the units per second that the asset_id has to pay
    fn get_units_per_second(asset_id: AssetId) -> Option<u128>;
}

/// convertion trait between AssetId and AssetLocation
pub trait AssetLocationGetter<AssetId, AssetLocation> {
    fn get_asset_location(asset_id: AssetId) -> Option<AssetLocation>;
    fn get_asset_id(location: AssetLocation) -> Option<AssetId>;
}
