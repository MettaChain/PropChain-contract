# milestone-crowdfunding

Crowdfunding where backers' money sits in escrow and is released in milestone tranches after backer approval. Closes #1441.

Milestones carry a description hash and a release share (bps summing to 10 000); backers vote with weight = contribution against a
quorum; rejection beyond the blocking threshold disputes the campaign; refunds open on failure, dispute, or a missed submission
window and pay each backer their share of the **unreleased** escrow. See the module docs in `src/lib.rs`. A dispute is only a state
change; the contract does not arbitrate it.

```bash
cargo test --manifest-path contract-extensions/contracts/milestone-crowdfunding/Cargo.toml
```
