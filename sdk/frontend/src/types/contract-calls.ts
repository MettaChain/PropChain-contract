/**
 * @propchain/sdk — Contract Call Types
 *
 * Comprehensive TypeScript interfaces for all contract method calls,
 * including parameters and return types for every public function.
 *
 * @module types/contract-calls
 */

// ============================================================================
// DEX Contract Calls
// ============================================================================

export interface CreatePoolParams {
  baseToken: number;
  quoteToken: number;
  baseReserve: bigint;
  quoteReserve: bigint;
  feePercentage: number;
}

export interface AddLiquidityParams {
  pairId: number;
  baseAmount: bigint;
  quoteAmount: bigint;
  minBaseAmount: bigint;
  minQuoteAmount: bigint;
}

export interface RemoveLiquidityParams {
  pairId: number;
  shares: bigint;
  minBaseAmount: bigint;
  minQuoteAmount: bigint;
}

export interface SwapParams {
  pairId: number;
  isBuyOrder: boolean;
  amountIn: bigint;
  minAmountOut: bigint;
}

export interface PlaceOrderParams {
  pairId: number;
  isBuyOrder: boolean;
  amount: bigint;
  limitPrice?: bigint;
  expiresAt: number;
}

export interface CancelOrderParams {
  orderId: number;
}

export interface InitiateCrossChainTradeParams {
  pairId: number;
  sourceChain: number;
  destinationChain: number;
  amount: bigint;
  expectedAmount: bigint;
  minReceivedAmount: bigint;
  expiresAt: number;
}

export interface CompleteCrossChainTradeParams {
  tradeId: number;
}

export interface CreateGovernanceTokenParams {
  symbol: string;
  totalSupply: bigint;
  emissionRate: bigint;
  quorumBips: number;
}

export interface GetPoolParams {
  pairId: number;
}

export interface GetLiquidityPositionParams {
  pairId: number;
  provider: string;
}

export interface GetOrderParams {
  orderId: number;
}

export interface GetAnalyticsParams {
  pairId: number;
}

export interface GetQuoteParams {
  pairId: number;
  isBuyOrder: boolean;
  amount: bigint;
}

export interface DexQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Lending Protocol Calls
// ============================================================================

export interface CreateLendingPoolParams {
  collateralToken: number;
  underlyingToken: number;
  borrowRate: number;
  supplyRate: number;
  maxLoanToValue: number;
}

export interface DepositParams {
  poolId: number;
  amount: bigint;
}

export interface WithdrawParams {
  poolId: number;
  amount: bigint;
}

export interface BorrowParams {
  poolId: number;
  borrowAmount: bigint;
  collateralAmount: bigint;
}

export interface RepayParams {
  positionId: number;
  amount: bigint;
}

export interface RequestFlashLoanParams {
  poolId: number;
  amount: bigint;
}

export interface UpdateInterestRateParams {
  poolId: number;
  newBorrowRate: number;
  newSupplyRate: number;
}

export interface SetMaxLTVParams {
  poolId: number;
  newMaxLTV: number;
}

export interface GetPoolParams {
  poolId: number;
}

export interface GetLendingPositionParams {
  positionId: number;
}

export interface GetBorrowingPositionParams {
  positionId: number;
}

export interface GetAccountLendingPositionsParams {
  account: string;
}

export interface GetAccountBorrowingPositionsParams {
  account: string;
}

export interface LendingQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Governance Contract Calls
// ============================================================================

export interface CreateProposalParams {
  title: string;
  description: string;
  executionCode: string;
  votingPeriodDays: number;
}

export interface CastVoteParams {
  proposalId: number;
  support: number; // 0: Against, 1: For, 2: Abstain
  reason?: string;
}

export interface QueueProposalParams {
  proposalId: number;
}

export interface ExecuteProposalParams {
  proposalId: number;
}

export interface CancelProposalParams {
  proposalId: number;
  reason: string;
}

export interface DelegateVotesParams {
  delegate: string;
}

export interface UndelegateVotesParams {}

export interface GetProposalParams {
  proposalId: number;
}

export interface GetProposalVotesParams {
  proposalId: number;
}

export interface GetVoterParams {
  account: string;
}

export interface GetVotesCastParams {
  proposalId: number;
  voter: string;
}

export interface GetDelegateParams {
  account: string;
}

export interface GovernanceQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Insurance Contract Calls
// ============================================================================

export interface CreatePolicyParams {
  propertyId: number;
  coverageAmount: bigint;
  premiumPerMonth: bigint;
  coverageType: string;
  durationMonths: number;
}

export interface PayPremiumParams {
  policyId: number;
  months: number;
}

export interface SubmitClaimParams {
  policyId: number;
  amount: bigint;
  description: string;
  evidence: string[];
}

export interface ApproveClaimParams {
  claimId: number;
  approvedAmount: bigint;
}

export interface RejectClaimParams {
  claimId: number;
  reason: string;
}

export interface PayClaimParams {
  claimId: number;
}

export interface RenewPolicyParams {
  policyId: number;
  durationMonths: number;
}

export interface CancelPolicyParams {
  policyId: number;
  reason: string;
}

export interface GetPolicyParams {
  policyId: number;
}

export interface GetClaimParams {
  claimId: number;
}

export interface GetPoliciesForPropertyParams {
  propertyId: number;
}

export interface GetPoliciesForHolderParams {
  holder: string;
}

export interface InsuranceQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Staking Contract Calls
// ============================================================================

export interface StakeParams {
  amount: bigint;
  lockDurationDays: number;
}

export interface UnstakeParams {
  positionId: number;
}

export interface ClaimRewardsParams {
  positionId?: number;
}

export interface DelegateToValidatorParams {
  validator: string;
  amount: bigint;
}

export interface UndelegateParams {
  validator: string;
  amount: bigint;
}

export interface RegisterValidatorParams {
  commissionPercentage: number;
}

export interface UpdateValidatorCommissionParams {
  newCommission: number;
}

export interface GetStakingPositionParams {
  positionId: number;
}

export interface GetStakerPositionsParams {
  staker: string;
}

export interface GetValidatorParams {
  validator: string;
}

export interface GetValidatorDelegationsParams {
  validator: string;
}

export interface GetDelegationParams {
  delegator: string;
  validator: string;
}

export interface CalculateRewardsParams {
  positionId: number;
}

export interface StakingQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Fractional Ownership Calls
// ============================================================================

export interface CreateOfferingParams {
  propertyId: number;
  totalShares: bigint;
  pricePerShare: bigint;
  minSharesPerBuy: bigint;
  maxSharesPerBuyer: bigint;
  offeringDurationDays: number;
}

export interface BuySharesParams {
  offeringId: number;
  shareCount: bigint;
}

export interface SellSharesParams {
  propertyId: number;
  sharesOffered: bigint;
  pricePerShare: bigint;
  expiresAt: number;
}

export interface BuySharesFromSellerParams {
  orderId: number;
  shareCount: bigint;
}

export interface ClaimDividendParams {
  propertyId: number;
  distributionId: number;
}

export interface GetOfferingParams {
  offeringId: number;
}

export interface GetShareholdersParams {
  propertyId: number;
}

export interface GetShareholdingParams {
  propertyId: number;
  shareholder: string;
}

export interface GetShareOrderParams {
  orderId: number;
}

export interface GetDividendDistributionParams {
  distributionId: number;
}

export interface FractionalQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Prediction Market Calls
// ============================================================================

export interface CreateMarketParams {
  propertyId: number;
  question: string;
  description: string;
  outcomes: string[];
  resolutionDateUnix: number;
}

export interface BetOnOutcomeParams {
  marketId: number;
  outcomeId: number;
  amount: bigint;
}

export interface SellPredictionSharesParams {
  marketId: number;
  outcomeId: number;
  shares: bigint;
}

export interface ResolveMarketParams {
  marketId: number;
  winningOutcomeId: number;
  resolution: string;
}

export interface ClaimWinningsParams {
  marketId: number;
  outcomeIds: number[];
}

export interface GetMarketParams {
  marketId: number;
}

export interface GetMarketOutcomesParams {
  marketId: number;
}

export interface GetPositionsParams {
  marketId: number;
  bettor: string;
}

export interface GetCurrentOddsParams {
  marketId: number;
}

export interface PredictionQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Crowdfunding Calls
// ============================================================================

export interface CreateCampaignParams {
  propertyId: number;
  targetAmount: bigint;
  deadlineUnix: number;
  minContribution: bigint;
  maxContributorsPerProperty: number;
}

export interface ContributeParams {
  campaignId: number;
  amount: bigint;
}

export interface RequestRefundParams {
  campaignId: number;
  contributionId: number;
}

export interface AddMilestoneParams {
  campaignId: number;
  title: string;
  description: string;
  targetAmount: bigint;
  dueDateUnix: number;
}

export interface ReleaseMilestoneParams {
  milestoneId: number;
}

export interface GetCampaignParams {
  campaignId: number;
}

export interface GetContributionsParams {
  campaignId: number;
}

export interface GetContributorParams {
  campaignId: number;
  contributor: string;
}

export interface GetMilestonesParams {
  campaignId: number;
}

export interface CrowdfundingQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// ZK Compliance Calls
// ============================================================================

export interface SubmitZKProofParams {
  proofType: string;
  proofData: string;
}

export interface VerifyZKProofParams {
  proofId: number;
}

export interface UpdatePrivacyPreferencesParams {
  allowDataSharing: boolean;
  allowProofSharing: boolean;
  anonymizeTransactions: boolean;
  dataRetentionMonths: number;
}

export interface SetPrivacyControlsParams {
  dataExposureLevel: string;
  consentList: string[];
}

export interface GrantProofConsentParams {
  proofTypes: string[];
}

export interface RevokeProofConsentParams {
  proofTypes: string[];
}

export interface CreateComplianceCertificateParams {
  certificateType: string;
  validMonths: number;
  signature: string;
}

export interface VerifyAddressOwnershipParams {
  proofData: string;
}

export interface VerifyPropertyOwnershipParams {
  propertyId: number;
  proofData: string;
}

export interface GetZKProofParams {
  account: string;
  proofId: number;
}

export interface GetPrivacyPreferencesParams {
  account: string;
}

export interface GetPrivacyDashboardParams {
  account: string;
}

export interface GetComplianceStatusParams {
  account: string;
}

export interface ZKComplianceQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Property Management Calls
// ============================================================================

export interface CreateManagementAgreementParams {
  propertyId: number;
  manager: string;
  managementFeeBips: number;
  durationMonths: number;
}

export interface CreateMaintenanceRequestParams {
  propertyId: number;
  description: string;
  priority: string;
  estimatedCost: bigint;
}

export interface CompleteMaintenanceParams {
  requestId: number;
  actualCost: bigint;
}

export interface UpdateOccupancyParams {
  propertyId: number;
  isOccupied: boolean;
  tenant?: string;
  rentAmount?: bigint;
}

export interface CollectMaintenanceFeesParams {
  propertyId: number;
}

export interface GetManagementAgreementParams {
  propertyId: number;
}

export interface GetMaintenanceRequestsParams {
  propertyId: number;
}

export interface GetOccupancyStatusParams {
  propertyId: number;
}

export interface ManagementQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Fees & Taxation Calls
// ============================================================================

export interface CalculateFeeParams {
  transactionAmount: bigint;
  transactionType: string;
  account: string;
  timestamp: number;
}

export interface UpdateDynamicFeeParams {
  baseFeePercentage: number;
  volumeThreshold: bigint;
  volumeDiscount: number;
}

export interface CollectFeesParams {
  transactionId: number;
}

export interface CreateTaxRecordParams {
  account: string;
  propertyId: number;
  transactionType: string;
  amount: bigint;
  gainLoss: bigint;
}

export interface PayTaxParams {
  recordId: number;
  amount: bigint;
}

export interface GetTaxRecordsParams {
  account: string;
  startDate: number;
  endDate: number;
}

export interface FeesQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// AI Valuation Calls
// ============================================================================

export interface DeployModelParams {
  modelVersion: string;
  modelData: string;
}

export interface RequestValuationParams {
  propertyId: number;
  features: Record<string, number>;
}

export interface DetectDriftParams {
  modelVersion: string;
  windowSize: number;
}

export interface CreateABTestParams {
  modelVersionA: string;
  modelVersionB: string;
  testDurationDays: number;
  samplePercentage: number;
}

export interface ReportABTestResultsParams {
  testId: number;
  resultsData: string;
}

export interface GetModelVersionParams {
  version: string;
}

export interface GetValuationParams {
  propertyId: number;
}

export interface GetDriftDetectionParams {
  modelVersion: string;
}

export interface AIValuationQueryResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Generic Result Types
// ============================================================================

export interface TransactionResult {
  success: boolean;
  transactionHash?: string;
  blockNumber?: number;
  error?: string;
  gasUsed?: number;
}

export interface ContractCallResult<T> {
  success: boolean;
  data?: T;
  blockNumber?: number;
  error?: string;
}

export interface BatchCallResult<T> {
  success: boolean;
  results: ContractCallResult<T>[];
  error?: string;
}

// ============================================================================
// Contract Interaction Base Types
// ============================================================================

export interface ContractInteractionOptions {
  gasLimit?: number;
  value?: bigint;
  nonce?: number;
  waitForConfirmation?: boolean;
  confirmationBlocks?: number;
}

export interface QueryOptions {
  blockNumber?: number;
  caller?: string;
}

export interface BatchCallOptions {
  parallel?: boolean;
  maxRetries?: number;
  retryDelayMs?: number;
}

// ============================================================================
// Error Response Types
// ============================================================================

export interface ContractError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  transactionHash?: string;
  blockNumber?: number;
}

export interface ValidationError extends ContractError {
  code: "VALIDATION_ERROR";
  validationErrors: Array<{
    field: string;
    message: string;
  }>;
}

export interface TransactionError extends ContractError {
  code: "TRANSACTION_ERROR";
  reason: string;
  revertData?: string;
}

export interface NetworkError extends ContractError {
  code: "NETWORK_ERROR";
  endpoint: string;
  statusCode: number;
}
