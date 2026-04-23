/**
 * @propchain/sdk — TypeScript Type Definitions
 *
 * Complete TypeScript types mirroring all on-chain Rust structures
 * from PropChain smart contracts (PropertyRegistry, PropertyToken,
 * Oracle, Escrow, Governance, Bridge, and more).
 *
 * @module types
 */

// ============================================================================
// Core Property Types
// ============================================================================

/**
 * Property metadata submitted when registering a property.
 *
 * Maps to Rust: `propchain_traits::PropertyMetadata`
 */
export interface PropertyMetadata {
  /** Physical/geographic location of the property */
  location: string;
  /** Property size in square meters */
  size: number;
  /** Legal description or title reference */
  legalDescription: string;
  /** Property valuation in USD (with 8 decimals on-chain) */
  valuation: bigint;
  /** IPFS or URL pointer to supporting documents */
  documentsUrl: string;
}

/**
 * Full on-chain property information including registration metadata.
 *
 * Maps to Rust: `propchain_traits::PropertyInfo`
 */
export interface PropertyInfo {
  /** Unique property ID assigned on registration */
  id: number;
  /** Current owner's account ID (SS58 address) */
  owner: string;
  /** Property metadata (location, size, valuation, etc.) */
  metadata: PropertyMetadata;
  /** Block timestamp when the property was registered */
  registeredAt: number;
}

/**
 * Property type classification.
 *
 * Maps to Rust: `propchain_traits::PropertyType`
 */
export enum PropertyType {
  Residential = "Residential",
  Commercial = "Commercial",
  Industrial = "Industrial",
  Land = "Land",
  MultiFamily = "MultiFamily",
  Retail = "Retail",
  Office = "Office",
}

/**
 * Aggregate portfolio summary for an owner.
 *
 * Maps to Rust: `propchain_contracts::PortfolioSummary`
 */
export interface PortfolioSummary {
  propertyCount: number;
  totalValuation: bigint;
  averageValuation: bigint;
  totalSize: number;
  averageSize: number;
}

/**
 * Detailed portfolio including individual property entries.
 *
 * Maps to Rust: `propchain_contracts::PortfolioDetails`
 */
export interface PortfolioDetails {
  owner: string;
  properties: PortfolioProperty[];
  totalCount: number;
}

/**
 * Individual property within a portfolio view.
 *
 * Maps to Rust: `propchain_contracts::PortfolioProperty`
 */
export interface PortfolioProperty {
  id: number;
  location: string;
  size: number;
  valuation: bigint;
  registeredAt: number;
}

// ============================================================================
// Escrow Types
// ============================================================================

/**
 * Escrow information for a property transfer.
 *
 * Maps to Rust: `propchain_contracts::EscrowInfo`
 */
export interface EscrowInfo {
  id: number;
  propertyId: number;
  buyer: string;
  seller: string;
  amount: bigint;
  released: boolean;
}

/**
 * Approval type for multi-signature operations.
 *
 * Maps to Rust: `propchain_traits::ApprovalType`
 */
export enum ApprovalType {
  Release = "Release",
  Refund = "Refund",
  EmergencyOverride = "EmergencyOverride",
}

// ============================================================================
// Oracle / Valuation Types
// ============================================================================

/**
 * Valuation method used for a property.
 *
 * Maps to Rust: `propchain_traits::ValuationMethod`
 */
export enum ValuationMethod {
  Automated = "Automated",
  Manual = "Manual",
  MarketData = "MarketData",
  Hybrid = "Hybrid",
  AIValuation = "AIValuation",
}

/**
 * Property valuation from the oracle system.
 *
 * Maps to Rust: `propchain_traits::PropertyValuation`
 */
export interface PropertyValuation {
  propertyId: number;
  /** Valuation in USD with 8 decimal places */
  valuation: bigint;
  /** Confidence score 0–100 */
  confidenceScore: number;
  /** Number of oracle sources used */
  sourcesUsed: number;
  /** Block timestamp when last updated */
  lastUpdated: number;
  valuationMethod: ValuationMethod;
}

/**
 * Valuation with confidence interval and volatility data.
 *
 * Maps to Rust: `propchain_traits::ValuationWithConfidence`
 */
export interface ValuationWithConfidence {
  valuation: PropertyValuation;
  /** Market volatility index 0–100 */
  volatilityIndex: number;
  /** Min–max confidence interval */
  confidenceInterval: [bigint, bigint];
  /** Number of outlier sources detected */
  outlierSources: number;
}

/**
 * Market volatility metrics for a location/property-type pair.
 *
 * Maps to Rust: `propchain_traits::VolatilityMetrics`
 */
export interface VolatilityMetrics {
  propertyType: PropertyType;
  location: string;
  volatilityIndex: number;
  averagePriceChange: number;
  periodDays: number;
  lastUpdated: number;
}

/**
 * Price data from an external feed.
 *
 * Maps to Rust: `propchain_traits::PriceData`
 */
export interface PriceData {
  /** Price in USD with 8 decimals */
  price: bigint;
  timestamp: number;
  source: string;
}

/**
 * Oracle source type classification.
 *
 * Maps to Rust: `propchain_traits::OracleSourceType`
 */
export enum OracleSourceType {
  Chainlink = "Chainlink",
  Pyth = "Pyth",
  Substrate = "Substrate",
  Custom = "Custom",
  Manual = "Manual",
  AIModel = "AIModel",
}

/**
 * Oracle source configuration.
 *
 * Maps to Rust: `propchain_traits::OracleSource`
 */
export interface OracleSource {
  id: string;
  sourceType: OracleSourceType;
  address: string;
  isActive: boolean;
  weight: number;
  lastUpdated: number;
}

/**
 * Price alert configuration.
 *
 * Maps to Rust: `propchain_traits::PriceAlert`
 */
export interface PriceAlert {
  propertyId: number;
  thresholdPercentage: number;
  alertAddress: string;
  lastTriggered: number;
  isActive: boolean;
}

/**
 * Comparable property for AVM analysis.
 *
 * Maps to Rust: `propchain_traits::ComparableProperty`
 */
export interface ComparableProperty {
  propertyId: number;
  distanceKm: number;
  pricePerSqm: bigint;
  sizeSqm: number;
  saleDate: number;
  adjustmentFactor: number;
}

/**
 * Location-based adjustment factors.
 *
 * Maps to Rust: `propchain_traits::LocationAdjustment`
 */
export interface LocationAdjustment {
  locationCode: string;
  adjustmentPercentage: number;
  lastUpdated: number;
  confidenceScore: number;
}

/**
 * Market trend data.
 *
 * Maps to Rust: `propchain_traits::MarketTrend`
 */
export interface MarketTrend {
  propertyType: PropertyType;
  location: string;
  trendPercentage: number;
  periodMonths: number;
  lastUpdated: number;
}

// ============================================================================
// Badge / Verification Types
// ============================================================================

/**
 * Badge types for property verification.
 *
 * Maps to Rust: `propchain_contracts::BadgeType`
 */
export enum BadgeType {
  OwnerVerification = "OwnerVerification",
  DocumentVerification = "DocumentVerification",
  LegalCompliance = "LegalCompliance",
  PremiumListing = "PremiumListing",
}

/**
 * Badge information attached to a property.
 *
 * Maps to Rust: `propchain_contracts::Badge`
 */
export interface Badge {
  badgeType: BadgeType;
  issuedAt: number;
  issuedBy: string;
  expiresAt: number | null;
  metadataUrl: string;
  revoked: boolean;
  revokedAt: number | null;
  revocationReason: string;
}

/**
 * Verification status enumeration.
 *
 * Maps to Rust: `propchain_contracts::VerificationStatus`
 */
export enum VerificationStatus {
  Pending = "Pending",
  Approved = "Approved",
  Rejected = "Rejected",
}

/**
 * Verification request for a badge.
 *
 * Maps to Rust: `propchain_contracts::VerificationRequest`
 */
export interface VerificationRequest {
  id: number;
  propertyId: number;
  badgeType: BadgeType;
  requester: string;
  requestedAt: number;
  evidenceUrl: string;
  status: VerificationStatus;
  reviewedBy: string | null;
  reviewedAt: number | null;
}

/**
 * Appeal status enumeration.
 *
 * Maps to Rust: `propchain_contracts::AppealStatus`
 */
export enum AppealStatus {
  Pending = "Pending",
  Approved = "Approved",
  Rejected = "Rejected",
}

/**
 * Appeal for badge revocation.
 *
 * Maps to Rust: `propchain_contracts::Appeal`
 */
export interface Appeal {
  id: number;
  propertyId: number;
  badgeType: BadgeType;
  appellant: string;
  reason: string;
  submittedAt: number;
  status: AppealStatus;
  resolvedBy: string | null;
  resolvedAt: number | null;
  resolution: string;
}

// ============================================================================
// Cross-Chain Bridge Types
// ============================================================================

/**
 * Bridge operation status.
 *
 * Maps to Rust: `propchain_traits::BridgeOperationStatus`
 */
export enum BridgeOperationStatus {
  None = "None",
  Pending = "Pending",
  Locked = "Locked",
  InTransit = "InTransit",
  Completed = "Completed",
  Failed = "Failed",
  Recovering = "Recovering",
  Expired = "Expired",
}

/**
 * Bridge status for a token.
 *
 * Maps to Rust: `propchain_traits::BridgeStatus`
 */
export interface BridgeStatus {
  isLocked: boolean;
  sourceChain: number | null;
  destinationChain: number | null;
  lockedAt: number | null;
  bridgeRequestId: number | null;
  status: BridgeOperationStatus;
}

/**
 * Bridge monitoring information.
 *
 * Maps to Rust: `propchain_traits::BridgeMonitoringInfo`
 */
export interface BridgeMonitoringInfo {
  bridgeRequestId: number;
  tokenId: number;
  sourceChain: number;
  destinationChain: number;
  status: BridgeOperationStatus;
  createdAt: number;
  expiresAt: number | null;
  signaturesCollected: number;
  signaturesRequired: number;
  errorMessage: string | null;
}

/**
 * Bridge transaction record.
 *
 * Maps to Rust: `propchain_traits::BridgeTransaction`
 */
export interface BridgeTransaction {
  transactionId: number;
  tokenId: number;
  sourceChain: number;
  destinationChain: number;
  sender: string;
  recipient: string;
  transactionHash: string;
  timestamp: number;
  gasUsed: number;
  status: BridgeOperationStatus;
  metadata: PropertyMetadata;
}

/**
 * Multi-signature bridge request.
 *
 * Maps to Rust: `propchain_traits::MultisigBridgeRequest`
 */
export interface MultisigBridgeRequest {
  requestId: number;
  tokenId: number;
  sourceChain: number;
  destinationChain: number;
  sender: string;
  recipient: string;
  requiredSignatures: number;
  signatures: string[];
  createdAt: number;
  expiresAt: number | null;
  status: BridgeOperationStatus;
  metadata: PropertyMetadata;
}

/**
 * Bridge configuration.
 *
 * Maps to Rust: `propchain_traits::BridgeConfig`
 */
export interface BridgeConfig {
  supportedChains: number[];
  minSignaturesRequired: number;
  maxSignaturesRequired: number;
  defaultTimeoutBlocks: number;
  gasLimitPerBridge: number;
  emergencyPause: boolean;
  metadataPreservation: boolean;
}

/**
 * Chain-specific bridge information.
 *
 * Maps to Rust: `propchain_traits::ChainBridgeInfo`
 */
export interface ChainBridgeInfo {
  chainId: number;
  chainName: string;
  bridgeContractAddress: string | null;
  isActive: boolean;
  gasMultiplier: number;
  confirmationBlocks: number;
  supportedTokens: number[];
}

/**
 * Recovery action for failed bridges.
 *
 * Maps to Rust: `propchain_traits::RecoveryAction`
 */
export enum RecoveryAction {
  UnlockToken = "UnlockToken",
  RefundGas = "RefundGas",
  RetryBridge = "RetryBridge",
  CancelBridge = "CancelBridge",
}

// ============================================================================
// Fee Types
// ============================================================================

/**
 * Operation types for dynamic fee calculation.
 *
 * Maps to Rust: `propchain_traits::FeeOperation`
 */
export enum FeeOperation {
  RegisterProperty = "RegisterProperty",
  TransferProperty = "TransferProperty",
  UpdateMetadata = "UpdateMetadata",
  CreateEscrow = "CreateEscrow",
  ReleaseEscrow = "ReleaseEscrow",
  PremiumListingBid = "PremiumListingBid",
}

// ============================================================================
// Token / Governance Types
// ============================================================================

/**
 * Ownership transfer record for property tokens.
 *
 * Maps to Rust: `property_token::OwnershipTransfer`
 */
export interface OwnershipTransfer {
  from: string;
  to: string;
  timestamp: number;
  transactionHash: string;
}

/**
 * Compliance information for a property token.
 *
 * Maps to Rust: `property_token::ComplianceInfo`
 */
export interface ComplianceInfo {
  verified: boolean;
  verificationDate: number;
  verifier: string;
  complianceType: string;
}

/**
 * Legal document information attached to a property token.
 *
 * Maps to Rust: `property_token::DocumentInfo`
 */
export interface DocumentInfo {
  documentHash: string;
  documentType: string;
  uploadDate: number;
  uploader: string;
}

/**
 * Governance proposal status.
 *
 * Maps to Rust: `property_token::ProposalStatus`
 */
export enum ProposalStatus {
  Open = "Open",
  Executed = "Executed",
  Rejected = "Rejected",
  Closed = "Closed",
}

/**
 * Governance proposal for fractional property tokens.
 *
 * Maps to Rust: `property_token::Proposal`
 */
export interface Proposal {
  id: number;
  tokenId: number;
  descriptionHash: string;
  quorum: bigint;
  forVotes: bigint;
  againstVotes: bigint;
  status: ProposalStatus;
  createdAt: number;
}

/**
 * Sell ask on the secondary market.
 *
 * Maps to Rust: `property_token::Ask`
 */
export interface Ask {
  tokenId: number;
  seller: string;
  pricePerShare: bigint;
  amount: bigint;
  createdAt: number;
}

/**
 * Tax record for dividend tracking.
 *
 * Maps to Rust: `property_token::TaxRecord`
 */
export interface TaxRecord {
  dividendsReceived: bigint;
  sharesSold: bigint;
  proceeds: bigint;
}

// ============================================================================
// Health / Monitoring Types
// ============================================================================

/**
 * Contract health status.
 *
 * Maps to Rust: `propchain_contracts::HealthStatus`
 */
export interface HealthStatus {
  isHealthy: boolean;
  isPaused: boolean;
  contractVersion: number;
  propertyCount: number;
  escrowCount: number;
  hasOracle: boolean;
  hasComplianceRegistry: boolean;
  hasFeeManager: boolean;
  blockNumber: number;
  timestamp: number;
}

/**
 * Gas usage metrics.
 *
 * Maps to Rust: `propchain_contracts::GasMetrics`
 */
export interface GasMetrics {
  lastOperationGas: number;
  averageOperationGas: number;
  totalOperations: number;
  minGasUsed: number;
  maxGasUsed: number;
}

/**
 * Global analytics data.
 *
 * Maps to Rust: `propchain_contracts::GlobalAnalytics`
 */
export interface GlobalAnalytics {
  totalProperties: number;
  totalValuation: bigint;
  averageValuation: bigint;
  totalSize: number;
  averageSize: number;
  uniqueOwners: number;
}

/**
 * Batch operation configuration.
 *
 * Maps to Rust: `propchain_contracts::BatchConfig`
 */
export interface BatchConfig {
  maxBatchSize: number;
  maxFailureThreshold: number;
}

/**
 * Result of a batch operation.
 *
 * Maps to Rust: `propchain_contracts::BatchResult`
 */
export interface BatchResult {
  successes: number[];
  failures: BatchItemFailure[];
  metrics: BatchMetrics;
}

/**
 * Single item failure in a batch operation.
 *
 * Maps to Rust: `propchain_contracts::BatchItemFailure`
 */
export interface BatchItemFailure {
  index: number;
  itemId: number;
  error: string;
}

/**
 * Metrics for a batch operation call.
 *
 * Maps to Rust: `propchain_contracts::BatchMetrics`
 */
export interface BatchMetrics {
  totalItems: number;
  successfulItems: number;
  failedItems: number;
  earlyTerminated: boolean;
}

/**
 * Historical batch operation statistics.
 *
 * Maps to Rust: `propchain_contracts::BatchOperationStats`
 */
export interface BatchOperationStats {
  totalBatchesProcessed: number;
  totalItemsProcessed: number;
  totalItemsFailed: number;
  totalEarlyTerminations: number;
  largestBatchProcessed: number;
}

// ============================================================================
// Pause Types
// ============================================================================

/**
 * Contract pause state and resume configuration.
 *
 * Maps to Rust: `propchain_contracts::PauseInfo`
 */
export interface PauseInfo {
  paused: boolean;
  pausedAt: number | null;
  pausedBy: string | null;
  reason: string | null;
  autoResumeAt: number | null;
  resumeRequestActive: boolean;
  resumeRequester: string | null;
  resumeApprovals: string[];
  requiredApprovals: number;
}

// ============================================================================
// Fractional Ownership Types
// ============================================================================

/**
 * Fractional ownership information.
 *
 * Maps to Rust: `propchain_contracts::FractionalInfo`
 */
export interface FractionalInfo {
  totalShares: bigint;
  enabled: boolean;
  createdAt: number;
}

// ============================================================================
// Error Types
// ============================================================================

/**
 * PropertyRegistry contract error codes.
 *
 * Maps to Rust: `propchain_contracts::Error`
 */
export enum PropertyRegistryError {
  PropertyNotFound = "PropertyNotFound",
  Unauthorized = "Unauthorized",
  InvalidMetadata = "InvalidMetadata",
  NotCompliant = "NotCompliant",
  ComplianceCheckFailed = "ComplianceCheckFailed",
  EscrowNotFound = "EscrowNotFound",
  EscrowAlreadyReleased = "EscrowAlreadyReleased",
  BadgeNotFound = "BadgeNotFound",
  InvalidBadgeType = "InvalidBadgeType",
  BadgeAlreadyIssued = "BadgeAlreadyIssued",
  NotVerifier = "NotVerifier",
  AppealNotFound = "AppealNotFound",
  InvalidAppealStatus = "InvalidAppealStatus",
  ComplianceRegistryNotSet = "ComplianceRegistryNotSet",
  OracleError = "OracleError",
  ContractPaused = "ContractPaused",
  AlreadyPaused = "AlreadyPaused",
  NotPaused = "NotPaused",
  ResumeRequestAlreadyActive = "ResumeRequestAlreadyActive",
  ResumeRequestNotFound = "ResumeRequestNotFound",
  InsufficientApprovals = "InsufficientApprovals",
  AlreadyApproved = "AlreadyApproved",
  NotAuthorizedToPause = "NotAuthorizedToPause",
  ZeroAddress = "ZeroAddress",
  StringTooLong = "StringTooLong",
  StringEmpty = "StringEmpty",
  ValueOutOfBounds = "ValueOutOfBounds",
  BatchSizeExceeded = "BatchSizeExceeded",
  SelfTransferNotAllowed = "SelfTransferNotAllowed",
  InvalidRange = "InvalidRange",
}

/**
 * PropertyToken contract error codes.
 *
 * Maps to Rust: `property_token::Error`
 */
export enum PropertyTokenError {
  TokenNotFound = "TokenNotFound",
  Unauthorized = "Unauthorized",
  PropertyNotFound = "PropertyNotFound",
  InvalidMetadata = "InvalidMetadata",
  DocumentNotFound = "DocumentNotFound",
  ComplianceFailed = "ComplianceFailed",
  BridgeNotSupported = "BridgeNotSupported",
  InvalidChain = "InvalidChain",
  BridgeLocked = "BridgeLocked",
  InsufficientSignatures = "InsufficientSignatures",
  RequestExpired = "RequestExpired",
  InvalidRequest = "InvalidRequest",
  BridgePaused = "BridgePaused",
  GasLimitExceeded = "GasLimitExceeded",
  MetadataCorruption = "MetadataCorruption",
  InvalidBridgeOperator = "InvalidBridgeOperator",
  DuplicateBridgeRequest = "DuplicateBridgeRequest",
  BridgeTimeout = "BridgeTimeout",
  AlreadySigned = "AlreadySigned",
  InsufficientBalance = "InsufficientBalance",
  InvalidAmount = "InvalidAmount",
  ProposalNotFound = "ProposalNotFound",
  ProposalClosed = "ProposalClosed",
  AskNotFound = "AskNotFound",
  BatchSizeExceeded = "BatchSizeExceeded",
}

/**
 * Oracle contract error codes.
 *
 * Maps to Rust: `propchain_traits::OracleError`
 */
export enum OracleErrorCode {
  PropertyNotFound = "PropertyNotFound",
  InsufficientSources = "InsufficientSources",
  InvalidValuation = "InvalidValuation",
  Unauthorized = "Unauthorized",
  OracleSourceNotFound = "OracleSourceNotFound",
  InvalidParameters = "InvalidParameters",
  PriceFeedError = "PriceFeedError",
  AlertNotFound = "AlertNotFound",
  InsufficientReputation = "InsufficientReputation",
  SourceAlreadyExists = "SourceAlreadyExists",
  RequestPending = "RequestPending",
  BatchSizeExceeded = "BatchSizeExceeded",
}

// ============================================================================
// SDK-specific Types
// ============================================================================

/**
 * Options for creating a PropChainClient.
 */
export interface ClientOptions {
  /** Custom types to register with the API */
  types?: Record<string, unknown>;
  /** Auto-reconnect on disconnect (default: true) */
  autoReconnect?: boolean;
  /** Maximum reconnection attempts (default: 5) */
  maxReconnectAttempts?: number;
  /** Connection timeout in milliseconds (default: 30000) */
  connectionTimeout?: number;
  /** Gas buffer percentage to add to estimates (e.g. 15 for 15%) (default: 10) */
  gasBufferPercentage?: number;
  /** Automatically adjust gas buffers based on network congestion (default: false) */
  autoAdjustGas?: boolean;
}

/**
 * Contract addresses for a deployment.
 */
export interface ContractAddresses {
  /** PropertyRegistry contract address */
  propertyRegistry?: string;
  /** PropertyToken contract address */
  propertyToken?: string;
  /** Oracle contract address */
  oracle?: string;
  /** Escrow contract address */
  escrow?: string;
}

/**
 * Result of a state-mutating transaction.
 */
export interface TxResult {
  /** Transaction hash */
  txHash: string;
  /** Block hash the transaction was included in */
  blockHash: string;
  /** Block number */
  blockNumber: number;
  /** Events emitted by the transaction */
  events: ContractEvent[];
  /** Whether the transaction was successful */
  success: boolean;
}

/**
 * Generic contract event.
 */
export interface ContractEvent {
  /** Event name */
  name: string;
  /** Event arguments */
  args: Record<string, unknown>;
}

/**
 * Gas estimation result.
 */
export interface GasEstimation {
  /** Estimated gas required */
  gasRequired: bigint;
  /** Storage deposit required */
  storageDeposit: bigint;
}

/**
 * Predefined network configurations.
 */
export interface NetworkConfig {
  name: string;
  wsEndpoint: string;
  explorerUrl?: string;
  isTestnet: boolean;
}

/**
 * Subscription handle for event listeners.
 */
export interface Subscription {
  /** Unsubscribe from the event stream */
  unsubscribe: () => void;
}

// ============================================================================
// Re-export comprehensive contract types
// ============================================================================

// Export all comprehensive contract types from contracts.ts
export type {
  // DEX Types
  LiquidityPool,
  LiquidityPosition,
  TradingOrder,
  SwapExecution,
  PairAnalytics,
  CrossChainTradeIntent,
  BridgeFeeQuote,
  OrderStatus,
  CrossChainTradeStatus,

  // Lending Types
  LendingPool,
  LendingPosition,
  BorrowingPosition,
  LiquidationEvent,
  InterestRateModel,
  FlashLoanRequest,
  BorrowingStatus,

  // Governance Types
  GovernanceProposal,
  GovernanceTokenConfig,
  VoteDelegation,
  ProposalStatus,

  // Insurance Types
  InsurancePolicy,
  InsuranceClaim,
  InsurancePool,
  ReinsuranceAgreement,
  InsuranceCoverageType,
  ClaimStatus,

  // Staking Types
  StakingPosition,
  StakingPool,
  StakingDelegation,
  ValidatorInfo,
  UnstakingRequest,

  // Fractional Ownership Types
  FractionalOffering,
  Shareholder,
  ShareTradingOrder,
  DividendDistribution,
  OfferingStatus,
  ShareOrderStatus,

  // Prediction Market Types
  PredictionMarket,
  PredictionOutcome,
  PredictionPosition,
  MarketStatus,

  // Crowdfunding Types
  CrowdfundingCampaign,
  CrowdfundingContribution,
  CampaignMilestone,
  CampaignStatus,

  // Analytics Types
  PropertyMetrics,
  MarketIndex,
  RiskAssessment,
  RiskFactor,
  RiskLevel,

  // Fees & Taxation Types
  DynamicFeeConfig,
  FeeCalculation,
  TaxRecord,
  TaxPaymentStatus,

  // Property Management Types
  ManagementAgreement,
  MaintenanceRequest,
  OccupancyStatus,
  MaintenancePriority,
  MaintenanceStatus,

  // AI Valuation Types
  ModelVersion,
  ModelMetrics,
  DriftDetectionResult,
  AIValuationResult,
  DeploymentStatus,
  DriftDetectionMethod,
  DriftRecommendation,

  // ZK Compliance Types
  ZKProofSubmission,
  PrivacyPreferences,
  ComplianceCertificate,
  ZKProofType,

  // Database & Storage Types
  StorageRecord,
  EncryptionStatus,

  // IPFS & Metadata Types
  IPFSResource,
  IPFSDocument,

  // Third-Party Types
  ThirdPartyIntegration,
  ExternalDataFeed,
  AuthMethod,

  // Identity & Compliance Types
  IdentityVerification,
  KYCInfo,
  ComplianceRegistryEntry,
  ComplianceStatus,
  VerificationStatus,
} from "./contracts";

// Export all comprehensive event types from contract-events.ts
export type {
  // DEX Events
  PoolCreatedEvent,
  LiquidityAddedEvent,
  SwapExecutedEvent,

  // Lending Events
  DepositedEvent,
  BorrowedEvent,
  RepaidEvent,
  LiquidatedEvent,

  // Governance Events
  ProposalCreatedEvent,
  VoteCastEvent,
  ProposalExecutedEvent,

  // Insurance Events
  InsurancePolicyCreatedEvent,
  ClaimSubmittedEvent,
  ClaimApprovedEvent,
  ClaimPaidEvent,

  // Staking Events
  StakedEvent,
  UnstakedEvent,
  RewardsClaimedEvent,

  // Fractional Ownership Events
  SharesPurchasedEvent,
  DividendDistributedEvent,

  // Prediction Market Events
  PredictionBetPlacedEvent,
  MarketResolvedEvent,

  // Crowdfunding Events
  ContributionMadeEvent,
  CampaignFundedEvent,

  // ZK Compliance Events
  ZKProofSubmittedEvent,
  ZKProofVerifiedEvent,

  // Identity Events
  IdentityVerifiedEvent,

  // Generic Events
  AdminChangedEvent,
  PausedEvent,
  ResumedEvent,

  // Union Type
  PropChainEvent,
} from "./contract-events";

// Export all contract call types from contract-calls.ts
export type {
  // DEX Calls
  CreatePoolParams,
  AddLiquidityParams,
  SwapParams,
  PlaceOrderParams,

  // Lending Calls
  DepositParams,
  BorrowParams,
  RepayParams,

  // Governance Calls
  CreateProposalParams,
  CastVoteParams,

  // Insurance Calls
  CreatePolicyParams,
  SubmitClaimParams,

  // Staking Calls
  StakeParams,
  DelegateToValidatorParams,

  // ZK Compliance Calls
  SubmitZKProofParams,
  VerifyZKProofParams,
  UpdatePrivacyPreferencesParams,

  // Generic Result Types
  TransactionResult,
  ContractCallResult,
  ContractError,
  ValidationError,
  TransactionError,
} from "./contract-calls";
