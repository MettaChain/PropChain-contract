// SPDX-License-Identifier: MIT

/// Trait for reading and writing off-chain metadata URIs for property tokens.
///
/// Metadata URIs typically point to IPFS or Arweave content containing
/// JSON documents with property descriptions, images, and attributes.
pub trait PropertyTokenMetadataTrait {
    /// Return the metadata URI for a given `token_id`, or `None` if unset.
    fn get_property_metadata(&self, token_id: u128) -> Option<String>;
    /// Set the metadata URI for a `token_id`. Returns an error on failure.
    fn set_property_metadata(&mut self, token_id: u128, metadata_uri: String) -> Result<(), String>;
}
