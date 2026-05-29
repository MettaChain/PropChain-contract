use crate::{LegalDocumentRecord, LegalDocumentStatus, LegalDocumentType, Timestamp};

pub(crate) fn build_document_record(
    property_id: u64,
    jurisdiction_code: u32,
    document_type: LegalDocumentType,
    document_hash: [u8; 32],
    issued_at: Timestamp,
    expires_at: Timestamp,
    verified: bool,
    now: Timestamp,
) -> LegalDocumentRecord {
    let status = if expires_at != 0 && expires_at <= now {
        LegalDocumentStatus::Expired
    } else if verified {
        LegalDocumentStatus::Verified
    } else {
        LegalDocumentStatus::Pending
    };

    LegalDocumentRecord {
        property_id,
        jurisdiction_code,
        document_type,
        document_hash,
        issued_at,
        expires_at,
        verified_at: if verified && status == LegalDocumentStatus::Verified {
            now
        } else {
            0
        },
        status,
    }
}

pub(crate) fn assessment_verified(record: &LegalDocumentRecord) -> bool {
    record.status == LegalDocumentStatus::Verified
}
