/**
 * @propchain/sdk — Contract-Specific Type Definitions
 *
 * Comprehensive TypeScript types for all PropChain smart contract
 * interactions including DEX, Lending, Governance, Insurance, and more.
 *
 * @module types/contracts
 */

// ============================================================================
// DEX (Decentralized Exchange) Types
// ============================================================================

/** Liquidity pool for trading property tokens */
export interface LiquidityPool {
  pairId: number;
  baseToken: number;
  quoteToken: number;
  baseReserve: bigint;
  quoteReserve: bigint;
  totalShares: bigint;
  feePercentage: number;
  createdAt: number;
}

/** User's liquidity position in a pool */
export interface LiquidityPosition {
  pairId: number;
  provider: string;
  shares: bigint;
  baseDeposited: bigint;
  quoteDeposited: bigint;
  feesClaimed: bigint;
  lastClaimedBlock: number;
}

/** Trading order in the order book */
export interface TradingOrder {
  orderId: number;
  pairId: number;
  trader: string;
  isLimitOrder: boolean;
  isBuyOrder: boolean;
  amount: bigint;
  limitPrice?: bigint;
  createdAt: number;
  expiresAt: number;
  filled: bigint;
  status: OrderStatus;
}

export enum OrderStatus {
  Active = "Active",
  PartiallyFilled = "PartiallyFilled",
  Filled = "Filled",
  Cancelled = "Cancelled",
  Expired = "Expired",
}

/** Swap execution details */
export interface SwapExecution {
  pairId: number;
  trader: string;
  isBuyOrder: boolean;
  amountIn: bigint;
  amountOut: bigint;
  priceImpact: number;
  executedAt: number;
  transactionHash: string;
}

/** Analytics for a trading pair */
export interface PairAnalytics {
  pairId: number;
  baseToken: number;
  quoteToken: number;
  volume24h: bigint;
  volume7d: bigint;
  currentPrice: bigint;
  price24hChange: number;
  highPrice24h: bigint;
  lowPrice24h: bigint;
  trades24h: number;
  avgTradeSize: bigint;
  lastUpdated: number;
}

/** Cross-chain trade intent */
export interface CrossChainTradeIntent {
  tradeId: number;
  pairId: number;
  sourceChain: number;
  destinationChain: number;
  trader: string;
  baseToken: number;
  quoteToken: number;
  amount: bigint;
  expectedAmount: bigint;
  minReceivedAmount: bigint;
  createdAt: number;
  expiresAt: number;
  status: CrossChainTradeStatus;
}

export enum CrossChainTradeStatus {
  Pending = "Pending",
  Locked = "Locked",
  InTransit = "InTransit",
  Completed = "Completed",
  Failed = "Failed",
  Refunded = "Refunded",
}

/** Bridge fee quote for cross-chain trading */
export interface BridgeFeeQuote {
  chainId: number;
  baseFeeBips: number;
  variableFeeBips: number;
  minFee: bigint;
  maxFee: bigint;
  validUntil: number;
}

// ============================================================================
// Lending Protocol Types
// ============================================================================

/** Lending pool for borrowing against property collateral */
export interface LendingPool {
  poolId: number;
  collateralToken: number;
  underlyingToken: number;
  totalBorrowed: bigint;
  totalDeposited: bigint;
  borrowRate: number;
  supplyRate: number;
  utilizationRate: number;
  maxLoanToValue: number;
  lastUpdated: number;
}

/** Lending position (deposit) */
export interface LendingPosition {
  positionId: number;
  poolId: number;
  lender: string;
  deposited: bigint;
  earned: bigint;
  depositedAt: number;
}

/** Borrowing position with collateral */
export interface BorrowingPosition {
  positionId: number;
  poolId: number;
  borrower: string;
  borrowed: bigint;
  collateralAmount: bigint;
  collateralValue: bigint;
  interestAccrued: bigint;
  borrowedAt: number;
  maturityAt: number;
  status: BorrowingStatus;
  liquidationPrice: bigint;
}

export enum BorrowingStatus {
  Active = "Active",
  Overdue = "Overdue",
  Liquidated = "Liquidated",
  Repaid = "Repaid",
}

/** Liquidation event */
export interface LiquidationEvent {
  positionId: number;
  borrower: string;
  liquidator: string;
  collateralSeized: bigint;
  debtRepaid: bigint;
  liquidatedAt: number;
  profitEarned: bigint;
}

/** Interest rate model configuration */
export interface InterestRateModel {
  baseRate: number;
  slopeA: number;
  slopeB: number;
  optimalUtilization: number;
  description: string;
}

/** Flash loan parameters */
export interface FlashLoanRequest {
  poolId: number;
  amount: bigint;
  borrower: string;
  fee: bigint;
  requestedAt: number;
}

// ============================================================================
// Governance Types
// ============================================================================

/** Governance proposal */
export interface GovernanceProposal {
  proposalId: number;
  proposer: string;
  title: string;
  description: string;
  executionCode: string;
  startBlock: number;
  endBlock: number;
  forVotes: bigint;
  againstVotes: bigint;
  abstainVotes: bigint;
  status: ProposalStatus;
  createdAt: number;
  executedAt: number | null;
}

export enum ProposalStatus {
  Pending = "Pending",
  Active = "Active",
  Cancelled = "Cancelled",
  Defeated = "Defeated",
  Succeeded = "Succeeded",
  Queued = "Queued",
  Expired = "Expired",
  Executed = "Executed",
}

/** Voting configuration */
export interface GovernanceTokenConfig {
  totalSupply: bigint;
  votingDelay: number;
  votingPeriod: number;
  proposalThreshold: bigint;
  quorumPercentage: number;
  majorityPercentage: number;
}

/** Delegation of voting power */
export interface VoteDelegation {
  delegator: string;
  delegate: string;
  votingPower: bigint;
  delegatedAt: number;
}

// ============================================================================
// Insurance Types
// ============================================================================

/** Insurance policy for property protection */
export interface InsurancePolicy {
  policyId: number;
  propertyId: number;
  policyholder: string;
  coverageAmount: bigint;
  premiumPerMonth: bigint;
  coverageType: InsuranceCoverageType;
  startDate: number;
  endDate: number;
  active: boolean;
  claimsHistory: number[];
}

export enum InsuranceCoverageType {
  Fire = "Fire",
  Theft = "Theft",
  StructuralDamage = "StructuralDamage",
  Liability = "Liability",
  Comprehensive = "Comprehensive",
}

/** Insurance claim */
export interface InsuranceClaim {
  claimId: number;
  policyId: number;
  claimant: string;
  amount: bigint;
  description: string;
  evidence: string[];
  submittedAt: number;
  status: ClaimStatus;
  approvedAmount: bigint;
  approvedBy: string | null;
  approvedAt: number | null;
}

export enum ClaimStatus {
  Submitted = "Submitted",
  UnderReview = "UnderReview",
  Approved = "Approved",
  Rejected = "Rejected",
  Paid = "Paid",
}

/** Insurance pool fund management */
export interface InsurancePool {
  poolId: number;
  totalPremiums: bigint;
  totalClaims: bigint;
  totalReserves: bigint;
  participantCount: number;
  activeClaimsCount: number;
}

/** Reinsurance agreement */
export interface ReinsuranceAgreement {
  agreementId: number;
  reinsurer: string;
  coveragePercentage: number;
  minPoolSize: bigint;
  maxCoveragePerClaim: bigint;
  premiumShare: number;
  active: boolean;
}

// ============================================================================
// Staking Types
// ============================================================================

/** Staking position */
export interface StakingPosition {
  positionId: number;
  staker: string;
  stakedAmount: bigint;
  rewardDebt: bigint;
  stakedAt: number;
  unlockedAt: number | null;
  lockDuration: number;
  multiplier: number;
}

/** Staking pool */
export interface StakingPool {
  poolId: number;
  tokenId: number;
  totalStaked: bigint;
  rewardRate: number;
  minStakeAmount: bigint;
  maxStakeAmount: bigint;
  lockDuration: number;
  createdAt: number;
}

/** Delegation to validator */
export interface StakingDelegation {
  delegator: string;
  validator: string;
  amount: bigint;
  delegatedAt: number;
  undelegateAt: number | null;
  rewards: bigint;
  lastRewardClaimBlock: number;
}

/** Validator information */
export interface ValidatorInfo {
  validator: string;
  totalDelegated: bigint;
  delegatorCount: number;
  commissionPercentage: number;
  totalRewardsDistributed: bigint;
  active: boolean;
  joinedAt: number;
}

/** Unstaking request */
export interface UnstakingRequest {
  requestId: number;
  staker: string;
  amount: bigint;
  requestedAt: number;
  releaseAt: number;
  claimed: boolean;
}

// ============================================================================
// Fractional Ownership Types
// ============================================================================

/** Fractional share offering */
export interface FractionalOffering {
  offeringId: number;
  propertyId: number;
  issuer: string;
  totalShares: bigint;
  availableShares: bigint;
  pricePerShare: bigint;
  minSharesPerBuy: bigint;
  maxSharesPerBuyer: bigint;
  offeringStartsAt: number;
  offeringEndsAt: number;
  status: OfferingStatus;
}

export enum OfferingStatus {
  Pending = "Pending",
  Active = "Active",
  Completed = "Completed",
  Cancelled = "Cancelled",
}

/** Shareholder information */
export interface Shareholder {
  shareholder: string;
  propertyId: number;
  shareCount: bigint;
  sharePercentage: number;
  acquiredAt: number;
  currentValue: bigint;
}

/** Share trading order in secondary market */
export interface ShareTradingOrder {
  orderId: number;
  propertyId: number;
  seller: string;
  sharesOffered: bigint;
  pricePerShare: bigint;
  createdAt: number;
  expiresAt: number;
  bought: bigint;
  status: ShareOrderStatus;
}

export enum ShareOrderStatus {
  Active = "Active",
  PartiallyFilled = "PartiallyFilled",
  Filled = "Filled",
  Cancelled = "Cancelled",
  Expired = "Expired",
}

/** Dividend distribution */
export interface DividendDistribution {
  distributionId: number;
  propertyId: number;
  totalAmount: bigint;
  amountPerShare: bigint;
  paymentDate: number;
  distributedAt: number;
  recipientCount: number;
}

// ============================================================================
// Prediction Market Types
// ============================================================================

/** Prediction market for property prices */
export interface PredictionMarket {
  marketId: number;
  propertyId: number;
  marketCreator: string;
  question: string;
  description: string;
  resolutionDate: number;
  createdAt: number;
  resolved: boolean;
  resolvedAt: number | null;
  resolution: string | null;
  status: MarketStatus;
}

export enum MarketStatus {
  Open = "Open",
  Trading = "Trading",
  Resolved = "Resolved",
  Closed = "Closed",
  Cancelled = "Cancelled",
}

/** Prediction outcome */
export interface PredictionOutcome {
  outcomeId: number;
  marketId: number;
  title: string;
  description: string;
  odds: number;
  totalBet: bigint;
  participantCount: number;
  odds24hChange: number;
}

/** Prediction position */
export interface PredictionPosition {
  positionId: number;
  marketId: number;
  outcomeId: number;
  bettor: string;
  amount: bigint;
  shares: bigint;
  avgPrice: number;
  bettedAt: number;
}

// ============================================================================
// Crowdfunding Types
// ============================================================================

/** Property crowdfunding campaign */
export interface CrowdfundingCampaign {
  campaignId: number;
  propertyId: number;
  creator: string;
  targetAmount: bigint;
  currentAmount: bigint;
  deadline: number;
  minContribution: bigint;
  maxContributorsPerProperty: number;
  createdAt: number;
  status: CampaignStatus;
}

export enum CampaignStatus {
  Draft = "Draft",
  Active = "Active",
  Funded = "Funded",
  Cancelled = "Cancelled",
  Completed = "Completed",
}

/** Contribution to crowdfunding */
export interface CrowdfundingContribution {
  contributionId: number;
  campaignId: number;
  contributor: string;
  amount: bigint;
  contributedAt: number;
  refunded: boolean;
  refundedAt: number | null;
}

/** Milestone for campaign disbursement */
export interface CampaignMilestone {
  milestoneId: number;
  campaignId: number;
  title: string;
  description: string;
  targetAmount: bigint;
  releasePercentage: number;
  dueDate: number;
  completed: boolean;
  completedAt: number | null;
}

// ============================================================================
// Analytics & Monitoring Types
// ============================================================================

/** Property performance metrics */
export interface PropertyMetrics {
  propertyId: number;
  viewCount: number;
  inquiryCount: number;
  offersReceived: number;
  averageOfferPrice: bigint;
  transactionCount: number;
  lastViewedAt: number;
  lastUpdatedAt: number;
}

/** Market index data */
export interface MarketIndex {
  indexId: number;
  indexName: string;
  propertyType: string;
  location: string;
  baseValue: bigint;
  currentValue: bigint;
  change24h: number;
  change7d: number;
  change30d: number;
  change1y: number;
  calculatedAt: number;
}

/** Risk assessment for a property */
export interface RiskAssessment {
  assessmentId: number;
  propertyId: number;
  riskScore: number;
  factors: RiskFactor[];
  overall: RiskLevel;
  assessedAt: number;
}

export interface RiskFactor {
  name: string;
  score: number;
  weight: number;
  description: string;
}

export enum RiskLevel {
  VeryLow = "VeryLow",
  Low = "Low",
  Medium = "Medium",
  High = "High",
  VeryHigh = "VeryHigh",
}

// ============================================================================
// Fees & Taxation Types
// ============================================================================

/** Dynamic fee structure */
export interface DynamicFeeConfig {
  baseFeePercentage: number;
  volumeThreshold: bigint;
  volumeDiscount: number;
  timeMultiplier: number;
  riskMultiplier: number;
  lastUpdated: number;
}

/** Fee calculation result */
export interface FeeCalculation {
  baseFee: bigint;
  volumeAdjustment: bigint;
  timeAdjustment: bigint;
  riskAdjustment: bigint;
  totalFee: bigint;
  feePercentage: number;
}

/** Tax record for transaction */
export interface TaxRecord {
  recordId: number;
  account: string;
  propertyId: number;
  transactionType: string;
  amount: bigint;
  gainLoss: bigint;
  taxableAmount: bigint;
  taxRate: number;
  taxOwed: bigint;
  paymentStatus: TaxPaymentStatus;
  transactionAt: number;
}

export enum TaxPaymentStatus {
  Pending = "Pending",
  Calculated = "Calculated",
  Paid = "Paid",
  Disputed = "Disputed",
  Settled = "Settled",
}

// ============================================================================
// Property Management Types
// ============================================================================

/** Property management agreement */
export interface ManagementAgreement {
  agreementId: number;
  propertyId: number;
  owner: string;
  manager: string;
  managementFeeBips: number;
  startDate: number;
  endDate: number | null;
  active: boolean;
  maintenanceReserve: bigint;
}

/** Maintenance request */
export interface MaintenanceRequest {
  requestId: number;
  propertyId: number;
  requestor: string;
  description: string;
  priority: MaintenancePriority;
  estimatedCost: bigint;
  createdAt: number;
  status: MaintenanceStatus;
  completedAt: number | null;
}

export enum MaintenancePriority {
  Low = "Low",
  Medium = "Medium",
  High = "High",
  Critical = "Critical",
}

export enum MaintenanceStatus {
  Reported = "Reported",
  Acknowledged = "Acknowledged",
  InProgress = "InProgress",
  Completed = "Completed",
  Verified = "Verified",
}

/** Occupancy information */
export interface OccupancyStatus {
  propertyId: number;
  isOccupied: boolean;
  tenantName: string | null;
  leaseStartDate: number | null;
  leaseEndDate: number | null;
  rentAmount: bigint | null;
  lastUpdated: number;
}

// ============================================================================
// AI Valuation Types
// ============================================================================

/** ML model version info */
export interface ModelVersion {
  version: string;
  releaseDate: number;
  accuracy: number;
  f1Score: number;
  deploymentStatus: DeploymentStatus;
  performanceMetrics: ModelMetrics;
}

export enum DeploymentStatus {
  Development = "Development",
  Testing = "Testing",
  Staging = "Staging",
  Production = "Production",
  Deprecated = "Deprecated",
}

export interface ModelMetrics {
  mae: number;
  rmse: number;
  r2Score: number;
  mape: number;
  testAccuracy: number;
}

/** Drift detection result */
export interface DriftDetectionResult {
  detectionTime: number;
  driftDetected: boolean;
  driftScore: number;
  driftMethod: DriftDetectionMethod;
  recommendation: DriftRecommendation;
  affectedProperties: number;
}

export enum DriftDetectionMethod {
  StatisticalTest = "StatisticalTest",
  DomainClassifier = "DomainClassifier",
  FeatureDistributionShift = "FeatureDistributionShift",
  LabelShift = "LabelShift",
}

export enum DriftRecommendation {
  Monitor = "Monitor",
  Retrain = "Retrain",
  Rollback = "Rollback",
  UpdateModel = "UpdateModel",
}

/** AI valuation result */
export interface AIValuationResult {
  propertyId: number;
  predictedValuation: bigint;
  confidenceScore: number;
  valuationRange: [bigint, bigint];
  modelVersion: string;
  usedFeatures: string[];
  generatedAt: number;
  expiresAt: number;
}

// ============================================================================
// ZK Compliance Types
// ============================================================================

/** Zero-knowledge proof submission */
export interface ZKProofSubmission {
  proofId: number;
  account: string;
  proofType: ZKProofType;
  proofData: string;
  submittedAt: number;
  verified: boolean;
  verifier: string | null;
  verifiedAt: number | null;
}

export enum ZKProofType {
  AgeVerification = "AgeVerification",
  IncomeVerification = "IncomeVerification",
  AccreditedInvestor = "AccreditedInvestor",
  PropertyOwnership = "PropertyOwnership",
  AddressOwnership = "AddressOwnership",
  KYC = "KYC",
  AML = "AML",
}

/** Privacy preferences */
export interface PrivacyPreferences {
  account: string;
  allowDataSharing: boolean;
  allowProofSharing: boolean;
  anonymizeTransactions: boolean;
  dataRetentionMonths: number;
  lastUpdatedAt: number;
}

/** Compliance certificate */
export interface ComplianceCertificate {
  certificateId: number;
  account: string;
  certificateType: string;
  validFrom: number;
  validUntil: number;
  issuer: string;
  signature: string;
}

// ============================================================================
// Database / Storage Types
// ============================================================================

/** Data storage record */
export interface StorageRecord {
  recordId: number;
  storageKey: string;
  owner: string;
  data: string;
  dataType: string;
  encryption: EncryptionStatus;
  createdAt: number;
  lastAccessedAt: number;
  accessCount: number;
}

export enum EncryptionStatus {
  Unencrypted = "Unencrypted",
  Encrypted = "Encrypted",
  DoubleEncrypted = "DoubleEncrypted",
}

// ============================================================================
// Metadata & IPFS Types
// ============================================================================

/** IPFS resource reference */
export interface IPFSResource {
  hash: string;
  contentType: string;
  size: number;
  uploadedAt: number;
  uploadedBy: string;
  pinned: boolean;
  replicationFactor: number;
}

/** Document stored on IPFS */
export interface IPFSDocument {
  documentId: number;
  propertyId: number;
  ipfsHash: string;
  documentType: string;
  description: string;
  uploadedBy: string;
  uploadedAt: number;
  verified: boolean;
  verifiedBy: string | null;
}

// ============================================================================
// Third-Party Integration Types
// ============================================================================

/** Third-party integration configuration */
export interface ThirdPartyIntegration {
  integrationId: number;
  provider: string;
  apiEndpoint: string;
  authMethod: AuthMethod;
  active: boolean;
  rateLimitPerMinute: number;
  lastSyncAt: number;
}

export enum AuthMethod {
  APIKey = "APIKey",
  OAuth2 = "OAuth2",
  JWT = "JWT",
  BasicAuth = "BasicAuth",
}

/** External data feed */
export interface ExternalDataFeed {
  feedId: number;
  feedName: string;
  provider: string;
  dataType: string;
  updateFrequency: number;
  lastUpdatedAt: number;
  active: boolean;
}

// ============================================================================
// Identity & Compliance Types
// ============================================================================

/** Identity verification status */
export interface IdentityVerification {
  verificationId: number;
  account: string;
  verified: boolean;
  verificationDate: number | null;
  verifier: string | null;
  documentsSubmitted: string[];
  riskLevel: RiskLevel;
}

/** KYC (Know Your Customer) information */
export interface KYCInfo {
  kycId: number;
  account: string;
  fullName: string;
  dateOfBirth: number;
  nationality: string;
  residenceCountry: string;
  verificationStatus: VerificationStatus;
  lastUpdatedAt: number;
}

export enum VerificationStatus {
  Pending = "Pending",
  Approved = "Approved",
  Rejected = "Rejected",
  Expired = "Expired",
}

/** Compliance registry entry */
export interface ComplianceRegistryEntry {
  entryId: number;
  account: string;
  jurisdiction: string;
  complianceStatus: ComplianceStatus;
  restrictions: string[];
  lastAuditAt: number;
  nextAuditAt: number;
}

export enum ComplianceStatus {
  Compliant = "Compliant",
  NonCompliant = "NonCompliant",
  PendingReview = "PendingReview",
  Restricted = "Restricted",
}
