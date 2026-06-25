# Escrow Dispute Evidence Submission

## Overview
When an escrow enters `Disputed` status, both parties may submit evidence
(document hashes) to support their claim. An arbitrator reviews the evidence
and calls `resolve_dispute`.

## Evidence struct

```rust
#[derive(Debug, Clone, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct DisputeEvidence {
    pub escrow_id: u64,
    pub submitter: AccountId,
    pub document_hash: Hash,
    pub description: String,
    pub submitted_at: u64,
}
```

## API

| Function | Caller | Description |
|---|---|---|
| `submit_evidence(escrow_id, doc_hash, description)` | buyer or seller | Attaches a document hash to the dispute |
| `get_evidence(escrow_id)` | anyone | Returns all evidence for an escrow |
| `resolve_dispute(escrow_id, release_to_buyer)` | arbitrator | Finalises the dispute and releases funds |

## Constraints
- Evidence may only be submitted while `status == Disputed`.
- Maximum 10 evidence items per escrow (prevents storage bloat).
- `document_hash` must be a 32-byte IPFS CID hash.

## Storage key
```rust
DisputeEvidence(escrow_id: u64)  // persistent Vec<DisputeEvidence>
```