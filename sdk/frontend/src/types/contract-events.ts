/**
 * @propchain/sdk — Comprehensive Event Type Definitions
 *
 * Complete TypeScript interfaces for all contract events emitted by PropChain
 * smart contracts. Organized by contract domain.
 *
 * @module types/contract-events
 */

// ============================================================================
// DEX Events
// ============================================================================

export interface PoolCreatedEvent {
  pairId: number;
  baseToken: number;
  quoteToken: number;
  creator: string;
  timestamp: number;
  blockNumber: number;
}

export interface LiquidityAddedEvent {
  pairId: number;
  provider: string;
  baseAmount: bigint;
  quoteAmount: bigint;
  mintedShares: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface LiquidityRemovedEvent {
  pairId: number;
  provider: string;
  baseAmount: bigint;
  quoteAmount: bigint;
  burnedShares: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface SwapExecutedEvent {
  pairId: number;
  trader: string;
  isBuyOrder: boolean;
  amountIn: bigint;
  amountOut: bigint;
  executionPrice: bigint;
  priceImpact: number;
  timestamp: number;
  blockNumber: number;
  transactionHash: string;
}

export interface OrderPlacedEvent {
  orderId: number;
  pairId: number;
  trader: string;
  isBuyOrder: boolean;
  amount: bigint;
  limitPrice?: bigint;
  expiresAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface OrderCancelledEvent {
  orderId: number;
  pairId: number;
  trader: string;
  cancelledAt: number;
  reasonCode: string;
  timestamp: number;
  blockNumber: number;
}

export interface OrderFilledEvent {
  orderId: number;
  pairId: number;
  trader: string;
  filledAmount: bigint;
  executionPrice: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface CrossChainTradeCreatedEvent {
  tradeId: number;
  pairId: number;
  trader: string;
  sourceChain: number;
  destinationChain: number;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface CrossChainTradeCompletedEvent {
  tradeId: number;
  trader: string;
  receivedAmount: bigint;
  bridgeFee: bigint;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Lending Protocol Events
// ============================================================================

export interface LendingPoolCreatedEvent {
  poolId: number;
  collateralToken: number;
  underlyingToken: number;
  creator: string;
  timestamp: number;
  blockNumber: number;
}

export interface DepositedEvent {
  poolId: number;
  lender: string;
  amount: bigint;
  sharesIssued: bigint;
  depositRate: number;
  timestamp: number;
  blockNumber: number;
}

export interface WithdrawnEvent {
  poolId: number;
  lender: string;
  amount: bigint;
  sharesBurned: bigint;
  interestEarned: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface BorrowedEvent {
  positionId: number;
  poolId: number;
  borrower: string;
  borrowAmount: bigint;
  collateralAmount: bigint;
  interestRate: number;
  maturityDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface RepaidEvent {
  positionId: number;
  borrower: string;
  principal: bigint;
  interest: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface LiquidatedEvent {
  positionId: number;
  borrower: string;
  liquidator: string;
  collateralSeized: bigint;
  debtCovered: bigint;
  liquidationBonus: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface FlashLoanEvent {
  loanId: number;
  borrower: string;
  amount: bigint;
  fee: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface InterestRateUpdatedEvent {
  poolId: number;
  oldBorrowRate: number;
  newBorrowRate: number;
  oldSupplyRate: number;
  newSupplyRate: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Governance Events
// ============================================================================

export interface ProposalCreatedEvent {
  proposalId: number;
  proposer: string;
  title: string;
  startBlock: number;
  endBlock: number;
  timestamp: number;
  blockNumber: number;
}

export interface ProposalStartedEvent {
  proposalId: number;
  startBlock: number;
  endBlock: number;
  timestamp: number;
  blockNumber: number;
}

export interface VoteCastEvent {
  proposalId: number;
  voter: string;
  support: VoteType;
  votes: bigint;
  reason: string;
  timestamp: number;
  blockNumber: number;
}

export enum VoteType {
  Against = 0,
  For = 1,
  Abstain = 2,
}

export interface ProposalQueuedEvent {
  proposalId: number;
  eta: number;
  timestamp: number;
  blockNumber: number;
}

export interface ProposalExecutedEvent {
  proposalId: number;
  executedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ProposalCancelledEvent {
  proposalId: number;
  cancelledBy: string;
  reason: string;
  timestamp: number;
  blockNumber: number;
}

export interface DelegateChangedEvent {
  delegator: string;
  fromDelegate: string;
  toDelegate: string;
  timestamp: number;
  blockNumber: number;
}

export interface DelegateVotesChangedEvent {
  delegate: string;
  previousBalance: bigint;
  newBalance: bigint;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Insurance Events
// ============================================================================

export interface InsurancePolicyCreatedEvent {
  policyId: number;
  propertyId: number;
  policyholder: string;
  coverageAmount: bigint;
  premiumPerMonth: bigint;
  coverageType: string;
  startDate: number;
  endDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface PremiumPaidEvent {
  policyId: number;
  policyholder: string;
  premiumAmount: bigint;
  paidFor: number;
  timestamp: number;
  blockNumber: number;
}

export interface ClaimSubmittedEvent {
  claimId: number;
  policyId: number;
  claimant: string;
  claimAmount: bigint;
  submittedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ClaimApprovedEvent {
  claimId: number;
  approvedAmount: bigint;
  approvedBy: string;
  approvalDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface ClaimRejectedEvent {
  claimId: number;
  rejectionReason: string;
  rejectedBy: string;
  rejectionDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface ClaimPaidEvent {
  claimId: number;
  claimant: string;
  paidAmount: bigint;
  paidAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface PolicyRenewedEvent {
  policyId: number;
  policyholder: string;
  newEndDate: number;
  renewalPremium: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface PolicyCancelledEvent {
  policyId: number;
  policyholder: string;
  refundAmount: bigint;
  cancellationReason: string;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Staking Events
// ============================================================================

export interface StakingPositionCreatedEvent {
  positionId: number;
  staker: string;
  amount: bigint;
  lockDuration: number;
  multiplier: number;
  stakedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface StakedEvent {
  positionId: number;
  staker: string;
  amount: bigint;
  newTotal: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface UnstakedEvent {
  positionId: number;
  staker: string;
  amount: bigint;
  newTotal: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface RewardsClaimedEvent {
  staker: string;
  rewardAmount: bigint;
  claimedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface DelegationCreatedEvent {
  delegator: string;
  validator: string;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface UndelegationRequestedEvent {
  delegator: string;
  validator: string;
  amount: bigint;
  releaseAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ValidatorRegisteredEvent {
  validator: string;
  commissionPercentage: number;
  registeredAt: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Fractional Ownership Events
// ============================================================================

export interface FractionalOfferingCreatedEvent {
  offeringId: number;
  propertyId: number;
  issuer: string;
  totalShares: bigint;
  pricePerShare: bigint;
  offeringEndsAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface SharesPurchasedEvent {
  offeringId: number;
  propertyId: number;
  buyer: string;
  sharesPurchased: bigint;
  totalCost: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface SharesSoldEvent {
  propertyId: number;
  seller: string;
  buyer: string;
  sharesSold: bigint;
  totalPrice: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface DividendDistributedEvent {
  distributionId: number;
  propertyId: number;
  totalAmount: bigint;
  amountPerShare: bigint;
  recipientCount: number;
  timestamp: number;
  blockNumber: number;
}

export interface DividendClaimedEvent {
  propertyId: number;
  recipient: string;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface ShareTransferEvent {
  propertyId: number;
  from: string;
  to: string;
  shares: bigint;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Prediction Market Events
// ============================================================================

export interface PredictionMarketCreatedEvent {
  marketId: number;
  propertyId: number;
  creator: string;
  question: string;
  resolutionDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface PredictionBetPlacedEvent {
  positionId: number;
  marketId: number;
  outcomeId: number;
  bettor: string;
  amount: bigint;
  shares: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface PredictionOddsUpdatedEvent {
  marketId: number;
  outcomeId: number;
  oldOdds: number;
  newOdds: number;
  timestamp: number;
  blockNumber: number;
}

export interface MarketResolvedEvent {
  marketId: number;
  winningOutcomeId: number;
  resolution: string;
  resolvedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface WinningsClaimedEvent {
  marketId: number;
  bettor: string;
  winnings: bigint;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Crowdfunding Events
// ============================================================================

export interface CrowdfundingCampaignCreatedEvent {
  campaignId: number;
  propertyId: number;
  creator: string;
  targetAmount: bigint;
  deadline: number;
  timestamp: number;
  blockNumber: number;
}

export interface ContributionMadeEvent {
  campaignId: number;
  contributor: string;
  amount: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface CampaignFundedEvent {
  campaignId: number;
  totalRaised: bigint;
  fundedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ContributionRefundedEvent {
  campaignId: number;
  contributor: string;
  refundAmount: bigint;
  refundedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface MilestoneReleasedEvent {
  milestoneId: number;
  campaignId: number;
  releaseAmount: bigint;
  releasedAt: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Monitoring & Analytics Events
// ============================================================================

export interface PropertyMetricsUpdatedEvent {
  propertyId: number;
  viewCount: number;
  inquiryCount: number;
  transactionCount: number;
  timestamp: number;
  blockNumber: number;
}

export interface AlertTriggeredEvent {
  alertId: number;
  propertyId: number;
  alertType: string;
  severity: string;
  message: string;
  timestamp: number;
  blockNumber: number;
}

export interface HealthCheckEvent {
  componentName: string;
  status: string;
  latency: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Fees & Taxation Events
// ============================================================================

export interface FeeCalculatedEvent {
  transactionId: number;
  account: string;
  baseFee: bigint;
  totalFee: bigint;
  adjustments: string;
  timestamp: number;
  blockNumber: number;
}

export interface FeeCollectedEvent {
  transactionId: number;
  amount: bigint;
  recipient: string;
  timestamp: number;
  blockNumber: number;
}

export interface TaxRecordCreatedEvent {
  recordId: number;
  account: string;
  transactionType: string;
  amount: bigint;
  taxAmount: bigint;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Property Management Events
// ============================================================================

export interface ManagementAgreementCreatedEvent {
  agreementId: number;
  propertyId: number;
  owner: string;
  manager: string;
  managementFee: number;
  startDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface MaintenanceRequestCreatedEvent {
  requestId: number;
  propertyId: number;
  requestor: string;
  priority: string;
  estimatedCost: bigint;
  timestamp: number;
  blockNumber: number;
}

export interface MaintenanceCompletedEvent {
  requestId: number;
  actualCost: bigint;
  completedBy: string;
  completedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface OccupancyChangedEvent {
  propertyId: number;
  occupied: boolean;
  tenant: string | null;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// ZK Compliance Events
// ============================================================================

export interface ZKProofSubmittedEvent {
  proofId: number;
  account: string;
  proofType: string;
  submittedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ZKProofVerifiedEvent {
  proofId: number;
  verifier: string;
  verified: boolean;
  verifiedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface PrivacyPreferencesUpdatedEvent {
  account: string;
  allowDataSharing: boolean;
  allowProofSharing: boolean;
  updatedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ComplianceCertificateIssuedEvent {
  certificateId: number;
  account: string;
  certificateType: string;
  issuer: string;
  validFrom: number;
  validUntil: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Metadata & IPFS Events
// ============================================================================

export interface IPFSResourceUploadedEvent {
  resourceHash: string;
  uploader: string;
  contentType: string;
  size: number;
  timestamp: number;
  blockNumber: number;
}

export interface DocumentAttachedEvent {
  documentId: number;
  propertyId: number;
  ipfsHash: string;
  documentType: string;
  uploadedBy: string;
  timestamp: number;
  blockNumber: number;
}

export interface DocumentVerifiedEvent {
  documentId: number;
  verifier: string;
  verifiedAt: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Bridge Events
// ============================================================================

export interface BridgeLockEvent {
  tokenId: number;
  sourceChain: number;
  destinationChain: number;
  owner: string;
  timestamp: number;
  blockNumber: number;
}

export interface BridgeUnlockEvent {
  tokenId: number;
  destinationChain: number;
  recipient: string;
  timestamp: number;
  blockNumber: number;
}

export interface BridgeFailureEvent {
  requestId: number;
  reason: string;
  failedAt: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Identity & Compliance Events
// ============================================================================

export interface IdentityVerifiedEvent {
  account: string;
  verifier: string;
  verificationDate: number;
  timestamp: number;
  blockNumber: number;
}

export interface KYCStatusUpdatedEvent {
  account: string;
  status: string;
  updatedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface ComplianceStatusChangedEvent {
  account: string;
  jurisdiction: string;
  newStatus: string;
  changedAt: number;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Generic Contract Events
// ============================================================================

export interface ErrorLoggedEvent {
  errorCode: string;
  account: string;
  message: string;
  timestamp: number;
  blockNumber: number;
}

export interface ContractUpgradedEvent {
  oldVersion: string;
  newVersion: string;
  upgradedAt: number;
  timestamp: number;
  blockNumber: number;
}

export interface AdminChangedEvent {
  oldAdmin: string;
  newAdmin: string;
  changedBy: string;
  timestamp: number;
  blockNumber: number;
}

export interface PausedEvent {
  pauser: string;
  timestamp: number;
  blockNumber: number;
}

export interface ResumedEvent {
  unpauser: string;
  timestamp: number;
  blockNumber: number;
}

// ============================================================================
// Union Type for All Events
// ============================================================================

export type PropChainEvent =
  | PoolCreatedEvent
  | SwapExecutedEvent
  | LiquidityAddedEvent
  | DepositedEvent
  | BorrowedEvent
  | ProposalCreatedEvent
  | VoteCastEvent
  | InsurancePolicyCreatedEvent
  | ClaimSubmittedEvent
  | StakedEvent
  | SharesPurchasedEvent
  | PredictionBetPlacedEvent
  | ContributionMadeEvent
  | ZKProofSubmittedEvent
  | IdentityVerifiedEvent
  | AdminChangedEvent
  | PausedEvent;
