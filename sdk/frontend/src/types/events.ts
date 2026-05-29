/**
 * @propchain/sdk — Event Type Definitions
 *
 * TypeScript interfaces for all contract events emitted by PropChain
 * smart contracts. These map directly to the `#[ink(event)]` structs
 * in the Rust source code.
 *
 * @module types/events
 */

// ============================================================================
// PropertyRegistry Events
// ============================================================================

/** Emitted when the contract is initialised */
export interface ContractInitializedEvent {
  admin: string;
  contractVersion: number;
  timestamp: number;
  blockNumber: number;
}

/** Emitted when a property is registered */
export interface PropertyRegisteredEvent {
  propertyId: number;
  owner: string;
  eventVersion: number;
  location: string;
  size: number;
  valuation: bigint;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Emitted when property ownership is transferred */
export interface PropertyTransferredEvent {
  propertyId: number;
  from: string;
  to: string;
  eventVersion: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
  transferredBy: string;
}

/** Emitted when property metadata is updated */
export interface PropertyMetadataUpdatedEvent {
  propertyId: number;
  owner: string;
  eventVersion: number;
  oldLocation: string;
  newLocation: string;
  oldValuation: bigint;
  newValuation: bigint;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Emitted when an approval is granted */
export interface ApprovalGrantedEvent {
  propertyId: number;
  owner: string;
  approved: string;
  eventVersion: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Emitted when an approval is cleared */
export interface ApprovalClearedEvent {
  propertyId: number;
  owner: string;
  eventVersion: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Emitted when an escrow is created */
export interface EscrowCreatedEvent {
  escrowId: number;
  propertyId: number;
  buyer: string;
  seller: string;
  eventVersion: number;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Emitted when escrow is released */
export interface EscrowReleasedEvent {
  escrowId: number;
  propertyId: number;
  buyer: string;
  eventVersion: number;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
  releasedBy: string;
}

/** Emitted when escrow is refunded */
export interface EscrowRefundedEvent {
  escrowId: number;
  propertyId: number;
  seller: string;
  eventVersion: number;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
  refundedBy: string;
}

/** Emitted when admin is changed */
export interface AdminChangedEvent {
  oldAdmin: string;
  newAdmin: string;
  eventVersion: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
  changedBy: string;
}

/** Batch property registration event */
export interface BatchPropertyRegisteredEvent {
  owner: string;
  eventVersion: number;
  propertyIds: number[];
  count: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Batch property transfer event */
export interface BatchPropertyTransferredEvent {
  from: string;
  to: string;
  eventVersion: number;
  propertyIds: number[];
  count: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
  transferredBy: string;
}

/** Batch metadata update event */
export interface BatchMetadataUpdatedEvent {
  owner: string;
  eventVersion: number;
  propertyIds: number[];
  count: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Batch operation completed event */
export interface BatchOperationCompletedEvent {
  operationCode: number;
  caller: string;
  eventVersion: number;
  totalItems: number;
  successfulItems: number;
  failedItems: number;
  earlyTerminated: boolean;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Badge issued event */
export interface BadgeIssuedEvent {
  propertyId: number;
  badgeType: string;
  issuedBy: string;
  eventVersion: number;
  expiresAt: number | null;
  metadataUrl: string;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Badge revoked event */
export interface BadgeRevokedEvent {
  propertyId: number;
  badgeType: string;
  revokedBy: string;
  eventVersion: number;
  reason: string;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Verification requested event */
export interface VerificationRequestedEvent {
  requestId: number;
  propertyId: number;
  badgeType: string;
  requester: string;
  eventVersion: number;
  evidenceUrl: string;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

/** Verification reviewed event */
export interface VerificationReviewedEvent {
  requestId: number;
  propertyId: number;
  reviewer: string;
  approved: boolean;
  eventVersion: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

// ============================================================================
// PropertyToken Events
// ============================================================================

/** ERC-721 Transfer event */
export interface TransferEvent {
  from: string | null;
  to: string | null;
  id: number;
}

/** ERC-721 Approval event */
export interface ApprovalEvent {
  owner: string;
  spender: string;
  id: number;
}

/** ERC-721 ApprovalForAll event */
export interface ApprovalForAllEvent {
  owner: string;
  operator: string;
  approved: boolean;
}

/** Property token minted */
export interface PropertyTokenMintedEvent {
  tokenId: number;
  propertyId: number;
  owner: string;
}

/** Legal document attached to a token */
export interface LegalDocumentAttachedEvent {
  tokenId: number;
  documentHash: string;
  documentType: string;
}

/** Compliance verified for a token */
export interface ComplianceVerifiedEvent {
  tokenId: number;
  verified: boolean;
  verifier: string;
}

/** Token bridged to another chain */
export interface TokenBridgedEvent {
  tokenId: number;
  destinationChain: number;
  recipient: string;
  bridgeRequestId: number;
}

/** Bridge request created */
export interface BridgeRequestCreatedEvent {
  requestId: number;
  tokenId: number;
  sourceChain: number;
  destinationChain: number;
  requester: string;
}

/** Bridge request signed */
export interface BridgeRequestSignedEvent {
  requestId: number;
  signer: string;
  signaturesCollected: number;
  signaturesRequired: number;
}

/** Bridge executed */
export interface BridgeExecutedEvent {
  requestId: number;
  tokenId: number;
  transactionHash: string;
}

/** Bridge failed */
export interface BridgeFailedEvent {
  requestId: number;
  tokenId: number;
  error: string;
}

/** Shares issued event */
export interface SharesIssuedEvent {
  tokenId: number;
  to: string;
  amount: bigint;
}

/** Shares redeemed event */
export interface SharesRedeemedEvent {
  tokenId: number;
  from: string;
  amount: bigint;
}

/** Dividends deposited event */
export interface DividendsDepositedEvent {
  tokenId: number;
  amount: bigint;
  perShare: bigint;
}

/** Dividends withdrawn event */
export interface DividendsWithdrawnEvent {
  tokenId: number;
  account: string;
  amount: bigint;
}

/** Governance proposal created */
export interface ProposalCreatedEvent {
  tokenId: number;
  proposalId: number;
  quorum: bigint;
}

/** Vote cast event */
export interface VotedEvent {
  tokenId: number;
  proposalId: number;
  voter: string;
  support: boolean;
  weight: bigint;
}

/** Proposal executed event */
export interface ProposalExecutedEvent {
  tokenId: number;
  proposalId: number;
  passed: boolean;
}

/** Ask placed on secondary market */
export interface AskPlacedEvent {
  tokenId: number;
  seller: string;
  pricePerShare: bigint;
  amount: bigint;
}

/** Ask cancelled */
export interface AskCancelledEvent {
  tokenId: number;
  seller: string;
}

/** Shares purchased on secondary market */
export interface SharesPurchasedEvent {
  tokenId: number;
  seller: string;
  buyer: string;
  amount: bigint;
  pricePerShare: bigint;
}

/** Property management contract set */
export interface PropertyManagementContractSetEvent {
  contract: string | null;
}

/** Management agent assigned */
export interface ManagementAgentAssignedEvent {
  tokenId: number;
  agent: string;
}

/** Management agent cleared */
export interface ManagementAgentClearedEvent {
  tokenId: number;
}

// ============================================================================
// Event Name Union
// ============================================================================

/**
 * Union of all known event names for type-safe event subscriptions.
 */
export type PropChainEventName =
  // PropertyRegistry events
  | 'ContractInitialized'
  | 'PropertyRegistered'
  | 'PropertyTransferred'
  | 'PropertyMetadataUpdated'
  | 'ApprovalGranted'
  | 'ApprovalCleared'
  | 'EscrowCreated'
  | 'EscrowReleased'
  | 'EscrowRefunded'
  | 'AdminChanged'
  | 'BatchPropertyRegistered'
  | 'BatchPropertyTransferred'
  | 'BatchMetadataUpdated'
  | 'BatchOperationCompleted'
  | 'BadgeIssued'
  | 'BadgeRevoked'
  | 'VerificationRequested'
  | 'VerificationReviewed'
  // PropertyToken events
  | 'Transfer'
  | 'Approval'
  | 'ApprovalForAll'
  | 'PropertyTokenMinted'
  | 'LegalDocumentAttached'
  | 'ComplianceVerified'
  | 'TokenBridged'
  | 'BridgeRequestCreated'
  | 'BridgeRequestSigned'
  | 'BridgeExecuted'
  | 'BridgeFailed'
  | 'SharesIssued'
  | 'SharesRedeemed'
  | 'DividendsDeposited'
  | 'DividendsWithdrawn'
  | 'ProposalCreated'
  | 'Voted'
  | 'ProposalExecuted'
  | 'AskPlaced'
  | 'AskCancelled'
  | 'SharesPurchased'
  | 'PropertyManagementContractSet'
  | 'ManagementAgentAssigned'
  | 'ManagementAgentCleared';

/**
 * Map from event name to its typed payload.
 */
export interface PropChainEventMap {
  ContractInitialized: ContractInitializedEvent;
  PropertyRegistered: PropertyRegisteredEvent;
  PropertyTransferred: PropertyTransferredEvent;
  PropertyMetadataUpdated: PropertyMetadataUpdatedEvent;
  ApprovalGranted: ApprovalGrantedEvent;
  ApprovalCleared: ApprovalClearedEvent;
  EscrowCreated: EscrowCreatedEvent;
  EscrowReleased: EscrowReleasedEvent;
  EscrowRefunded: EscrowRefundedEvent;
  AdminChanged: AdminChangedEvent;
  BatchPropertyRegistered: BatchPropertyRegisteredEvent;
  BatchPropertyTransferred: BatchPropertyTransferredEvent;
  BatchMetadataUpdated: BatchMetadataUpdatedEvent;
  BatchOperationCompleted: BatchOperationCompletedEvent;
  BadgeIssued: BadgeIssuedEvent;
  BadgeRevoked: BadgeRevokedEvent;
  VerificationRequested: VerificationRequestedEvent;
  VerificationReviewed: VerificationReviewedEvent;
  Transfer: TransferEvent;
  Approval: ApprovalEvent;
  ApprovalForAll: ApprovalForAllEvent;
  PropertyTokenMinted: PropertyTokenMintedEvent;
  LegalDocumentAttached: LegalDocumentAttachedEvent;
  ComplianceVerified: ComplianceVerifiedEvent;
  TokenBridged: TokenBridgedEvent;
  BridgeRequestCreated: BridgeRequestCreatedEvent;
  BridgeRequestSigned: BridgeRequestSignedEvent;
  BridgeExecuted: BridgeExecutedEvent;
  BridgeFailed: BridgeFailedEvent;
  SharesIssued: SharesIssuedEvent;
  SharesRedeemed: SharesRedeemedEvent;
  DividendsDeposited: DividendsDepositedEvent;
  DividendsWithdrawn: DividendsWithdrawnEvent;
  ProposalCreated: ProposalCreatedEvent;
  Voted: VotedEvent;
  ProposalExecuted: ProposalExecutedEvent;
  AskPlaced: AskPlacedEvent;
  AskCancelled: AskCancelledEvent;
  SharesPurchased: SharesPurchasedEvent;
  PropertyManagementContractSet: PropertyManagementContractSetEvent;
  ManagementAgentAssigned: ManagementAgentAssignedEvent;
  ManagementAgentCleared: ManagementAgentClearedEvent;
}
