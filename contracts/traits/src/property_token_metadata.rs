pub trait PropertyTokenMetadataTrait {
    fn get_property_metadata(&self, token_id: u128) -> Option<String>;
    fn set_property_metadata(&mut self, token_id: u128, metadata_uri: String) -> Result<(), String>;
}
