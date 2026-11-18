//! Unit tests for the IPFS metadata registry: CID storage and retrieval, access
//! control, CID validation and bulk registration (#1464).
//!
//! The contract has no key/value metadata store and no document *delete*; the
//! tests map the issue's items onto what exists: documents are stored by CID and
//! keyed by property, "update" is pinning/unpinning and access grants, and the
//! closest thing to deletion is unpinning and revoking access.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;
    use ipfs_metadata::ipfs_metadata::{
        AccessLevel, DocumentType, Error, IpfsMetadataRegistry, ValidationRules,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const PROPERTY: u64 = 7;
    /// A syntactically valid CIDv0 (`Qm` + 44 characters = 46).
    const CID_V0: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

    fn setup() -> (IpfsMetadataRegistry, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        (IpfsMetadataRegistry::new(), accounts)
    }

    fn hash(byte: u8) -> Hash {
        Hash::from([byte; 32])
    }

    fn cid(n: u32) -> String {
        // 46 characters: "Qm" + 44 digits/letters derived from n.
        format!("Qm{:0>44}", n)
    }

    fn register(r: &mut IpfsMetadataRegistry, cid: &str) -> Result<u64, Error> {
        r.register_ipfs_document(
            PROPERTY,
            cid.to_string(),
            DocumentType::Deed,
            hash(1),
            1_000,
            "application/pdf".to_string(),
            false,
        )
    }

    // ── CID storage and retrieval ───────────────────────────────────────

    #[ink::test]
    fn stores_a_document_under_a_valid_cid_and_reads_it_back() {
        let (mut r, accounts) = setup();
        let id = register(&mut r, CID_V0).expect("valid CID is accepted");
        assert_eq!(id, 1);
        assert_eq!(r.document_count(), 1);

        let doc = r.get_document(id).expect("stored");
        assert_eq!(doc.ipfs_cid, CID_V0);
        assert_eq!(doc.property_id, PROPERTY);
        assert_eq!(doc.document_type, DocumentType::Deed);
        assert_eq!(doc.file_size, 1_000);
        assert_eq!(doc.uploader, accounts.alice);
        assert!(!doc.is_pinned);
    }

    #[ink::test]
    fn retrieval_by_cid_and_by_property_agree() {
        let (mut r, _) = setup();
        let id = register(&mut r, CID_V0).unwrap();
        assert_eq!(r.get_document_by_cid(CID_V0.to_string()).unwrap().document_id, id);
        assert_eq!(r.get_property_documents(PROPERTY), vec![id]);
        assert!(r.get_document_by_cid(cid(99)).is_none());
        assert!(r.get_document(42).is_none());
        assert!(r.get_property_documents(999).is_empty());
    }

    #[ink::test]
    fn the_same_cid_cannot_be_registered_twice() {
        let (mut r, _) = setup();
        register(&mut r, CID_V0).unwrap();
        assert_eq!(register(&mut r, CID_V0), Err(Error::DocumentAlreadyExists));
        assert_eq!(r.document_count(), 1);
    }

    // ── CID format validation ───────────────────────────────────────────

    #[ink::test]
    fn cid_validation_accepts_v0_and_v1_shapes() {
        let (r, _) = setup();
        assert_eq!(r.validate_ipfs_cid(CID_V0.to_string()), Ok(()));
        assert_eq!(r.validate_ipfs_cid("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string()), Ok(()));
        // CIDv1 boundary: minimum accepted length is 10.
        assert_eq!(r.validate_ipfs_cid("b123456789".to_string()), Ok(()));
    }

    #[ink::test]
    fn cid_validation_rejects_malformed_values() {
        let (r, _) = setup();
        for bad in [
            "",
            "Qm",                                             // too short for v0
            "QmShort",                                        // v0 must be 46 chars
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdGX", // 47 chars
            "b12345678",                                      // v1 below 10 chars
            "xyz1234567890",                                  // unknown prefix
        ] {
            assert_eq!(r.validate_ipfs_cid(bad.to_string()), Err(Error::InvalidIpfsCid), "{bad}");
        }
    }

    #[ink::test]
    fn registering_an_invalid_cid_stores_nothing() {
        let (mut r, _) = setup();
        assert_eq!(register(&mut r, "not-a-cid"), Err(Error::InvalidIpfsCid));
        assert_eq!(r.document_count(), 0);
        assert!(r.get_property_documents(PROPERTY).is_empty());
    }

    // ── Authorization ───────────────────────────────────────────────────

    #[ink::test]
    fn an_unauthorized_account_cannot_register_or_pin() {
        let (mut r, accounts) = setup();
        let id = register(&mut r, CID_V0).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(register(&mut r, &cid(2)), Err(Error::Unauthorized));
        assert_eq!(r.pin_document(id), Err(Error::Unauthorized));
        assert_eq!(r.unpin_document(id), Err(Error::Unauthorized));
        assert!(!r.get_document(id).unwrap().is_pinned);
    }

    #[ink::test]
    fn write_access_is_granted_and_revoked_by_the_admin() {
        let (mut r, accounts) = setup();
        r.grant_access(PROPERTY, accounts.bob, AccessLevel::Write).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(register(&mut r, CID_V0).is_ok());

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        r.revoke_access(PROPERTY, accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(register(&mut r, &cid(2)), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn read_only_access_does_not_allow_writes() {
        let (mut r, accounts) = setup();
        r.grant_access(PROPERTY, accounts.bob, AccessLevel::Read).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(register(&mut r, CID_V0), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn only_a_property_admin_or_the_contract_admin_can_grant_access() {
        let (mut r, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            r.grant_access(PROPERTY, accounts.charlie, AccessLevel::Write),
            Err(Error::Unauthorized)
        );
    }

    // ── Updating state: pin / unpin ─────────────────────────────────────

    #[ink::test]
    fn pinning_updates_the_document_and_the_pinned_size() {
        let (mut r, _) = setup();
        let id = register(&mut r, CID_V0).unwrap();
        r.pin_document(id).unwrap();
        assert!(r.get_document(id).unwrap().is_pinned);
        assert_eq!(r.get_property_pinned_size(PROPERTY), 1_000);

        // Pinning twice is a no-op.
        r.pin_document(id).unwrap();
        assert_eq!(r.get_property_pinned_size(PROPERTY), 1_000);

        r.unpin_document(id).unwrap();
        assert!(!r.get_document(id).unwrap().is_pinned);
        assert_eq!(r.get_property_pinned_size(PROPERTY), 0);
    }

    #[ink::test]
    fn pinning_an_unknown_document_fails() {
        let (mut r, _) = setup();
        assert_eq!(r.pin_document(5), Err(Error::DocumentNotFound));
    }

    #[ink::test]
    fn pinning_beyond_the_per_property_limit_is_rejected() {
        let (mut r, _) = setup();
        let mut rules: ValidationRules = r.get_validation_rules();
        rules.max_pinned_size_per_property = 1_500;
        r.update_validation_rules(rules).unwrap();

        let a = register(&mut r, &cid(1)).unwrap();
        let b = register(&mut r, &cid(2)).unwrap();
        r.pin_document(a).unwrap();
        assert_eq!(r.pin_document(b), Err(Error::PinLimitExceeded));
        assert!(!r.get_document(b).unwrap().is_pinned);
    }

    // ── Multiple documents / bulk registration ──────────────────────────

    #[ink::test]
    fn many_documents_for_one_property_keep_ids_in_order() {
        let (mut r, _) = setup();
        let ids: Vec<u64> = (1..=25).map(|n| register(&mut r, &cid(n)).unwrap()).collect();
        assert_eq!(ids, (1..=25).collect::<Vec<u64>>());
        assert_eq!(r.get_property_documents(PROPERTY), ids);
        assert_eq!(r.document_count(), 25);
    }

    #[ink::test]
    fn documents_of_different_properties_are_kept_separate() {
        let (mut r, _) = setup();
        r.register_ipfs_document(1, cid(1), DocumentType::Other, hash(1), 10, "text/plain".into(), false).unwrap();
        r.register_ipfs_document(2, cid(2), DocumentType::Other, hash(2), 10, "text/plain".into(), false).unwrap();
        assert_eq!(r.get_property_documents(1), vec![1]);
        assert_eq!(r.get_property_documents(2), vec![2]);
    }

    #[ink::test]
    fn the_per_property_document_cap_stops_a_bulk_upload() {
        let (mut r, _) = setup();
        let mut rules = r.get_validation_rules();
        rules.max_documents_per_property = 3;
        r.update_validation_rules(rules).unwrap();

        for n in 1..=3 {
            register(&mut r, &cid(n)).unwrap();
        }
        assert_eq!(register(&mut r, &cid(4)), Err(Error::SizeLimitExceeded));
        assert_eq!(r.get_property_documents(PROPERTY).len(), 3);
    }

    // ── Content rules ───────────────────────────────────────────────────

    #[ink::test]
    fn oversized_files_and_mismatched_mime_types_are_rejected() {
        let (mut r, _) = setup();
        assert_eq!(
            r.register_ipfs_document(PROPERTY, cid(1), DocumentType::Deed, hash(1), 100_000_001, "application/pdf".into(), false),
            Err(Error::SizeLimitExceeded)
        );
        // A deed must be a PDF-like encoding, not an image.
        assert_eq!(
            r.register_ipfs_document(PROPERTY, cid(2), DocumentType::Deed, hash(1), 10, "image/png".into(), false),
            Err(Error::FileTypeNotAllowed)
        );
        assert_eq!(r.document_count(), 0);
    }

    #[ink::test]
    fn the_admin_mime_allowlist_restricts_uploads_once_set() {
        let (mut r, accounts) = setup();
        r.add_allowed_mime_type("application/pdf".to_string()).unwrap();
        assert!(register(&mut r, CID_V0).is_ok());
        // `octet-stream` passes the type check for a deed but is not on the allowlist.
        assert_eq!(
            r.register_ipfs_document(PROPERTY, cid(2), DocumentType::Deed, hash(1), 10, "application/octet-stream".into(), false),
            Err(Error::FileTypeNotAllowed)
        );
        // Only the admin may extend the list.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(r.add_allowed_mime_type("image/png".to_string()), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn content_hash_verification_compares_against_the_stored_hash() {
        let (mut r, _) = setup();
        let id = register(&mut r, CID_V0).unwrap();
        assert_eq!(r.verify_content_hash(id, hash(1)), Ok(true));
        assert_eq!(r.verify_content_hash(id, hash(2)), Err(Error::ContentHashMismatch));
    }
}
