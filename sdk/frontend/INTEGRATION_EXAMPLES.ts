/**
 * @propchain/sdk — Complete Integration Examples
 *
 * Comprehensive TypeScript examples demonstrating real-world usage
 * of PropChain contract types across all major contracts.
 *
 * @module examples
 */

// ============================================================================
// Example 1: DEX Liquidity Management
// ============================================================================

import type {
  LiquidityPool,
  LiquidityPosition,
  AddLiquidityParams,
  SwapParams,
  SwapExecutedEvent,
} from "@propchain/sdk";

/**
 * Complete DEX interaction workflow with type safety
 */
async function dexWorkflow() {
  // Create pool parameters with full typing
  const createPoolParams = {
    baseToken: 1, // Property Token ID
    quoteToken: 2, // USDC or stablecoin
    baseReserve: BigInt("1000000000000000000"), // 1 property token
    quoteReserve: BigInt("500000000000000000"), // 0.5 stablecoin
    feePercentage: 30, // 0.3% (in basis points)
  };

  // Add liquidity with typed parameters
  const liquidityParams: AddLiquidityParams = {
    pairId: 1,
    baseAmount: BigInt("100000000000000000"),
    quoteAmount: BigInt("50000000000000000"),
    minBaseAmount: BigInt("99000000000000000"), // 1% slippage
    minQuoteAmount: BigInt("49000000000000000"),
  };

  // Execute swap with type safety
  const swapParams: SwapParams = {
    pairId: 1,
    isBuyOrder: true, // Buy base token
    amountIn: BigInt("50000000000000000"), // 0.5 stablecoin
    minAmountOut: BigInt("95000000000000000"), // 0.95 property tokens
  };

  // Listen for swap events with full typing
  // client.on('SwapExecuted', (event: SwapExecutedEvent) => {
  //   console.log(`Executed: ${event.amountIn} → ${event.amountOut}`);
  //   console.log(`Price impact: ${event.priceImpact}%`);
  // });
}

// ============================================================================
// Example 2: Lending Protocol - Deposit & Borrow
// ============================================================================

import type {
  LendingPool,
  LendingPosition,
  BorrowingPosition,
  DepositParams,
  BorrowParams,
  RepayParams,
  DepositedEvent,
  BorrowedEvent,
  LiquidatedEvent,
} from "@propchain/sdk";

/**
 * Complete lending workflow with collateral management
 */
async function lendingWorkflow() {
  // 1. Supply capital to earn interest
  const depositParams: DepositParams = {
    poolId: 1,
    amount: BigInt("10000000000000000000"), // 10 stablecoin
  };

  // 2. Borrow against property collateral
  const borrowParams: BorrowParams = {
    poolId: 1,
    borrowAmount: BigInt("5000000000000000000"), // 5 stablecoin
    collateralAmount: BigInt("100000000000000000"), // 1 property token
  };

  // 3. Repay loan with interest
  const repayParams: RepayParams = {
    positionId: 1,
    amount: BigInt("5100000000000000000"), // 5 + 0.1 interest
  };

  // Track events with full type safety
  // client.on('Deposited', (event: DepositedEvent) => {
  //   console.log(`Deposited: ${event.amount}`);
  //   console.log(`Earned so far: ${event.depositRate}%`);
  // });

  // client.on('Borrowed', (event: BorrowedEvent) => {
  //   console.log(`Borrowed: ${event.borrowAmount}`);
  //   console.log(`Interest rate: ${event.interestRate}%`);
  //   console.log(`Maturity: ${new Date(event.maturityDate * 1000)}`);
  // });

  // client.on('Liquidated', (event: LiquidatedEvent) => {
  //   console.log(`Position liquidated!`);
  //   console.log(`Collateral seized: ${event.collateralSeized}`);
  //   console.log(`Liquidation bonus: ${event.liquidationBonus}`);
  // });
}

// ============================================================================
// Example 3: Governance - Create & Vote on Proposals
// ============================================================================

import type {
  GovernanceProposal,
  CreateProposalParams,
  CastVoteParams,
  ProposalStatus,
  VoteCastEvent,
  ProposalExecutedEvent,
} from "@propchain/sdk";

/**
 * Complete governance workflow with proposal execution
 */
async function governanceWorkflow() {
  // Create a proposal to update insurance parameters
  const proposalParams: CreateProposalParams = {
    title: "Increase Maximum Insurance Payout",
    description: `
      This proposal seeks to increase the maximum insurance payout per claim
      from 1,000,000 to 5,000,000 tokens to better cover property damages.
      
      Rationale: Recent market conditions show increased property values.
    `,
    executionCode: "insurance_set_max_payout(5000000000000000000)",
    votingPeriodDays: 7,
  };

  // Cast a vote with full type safety
  const voteParams: CastVoteParams = {
    proposalId: 1,
    support: 1, // 0 = Against, 1 = For, 2 = Abstain
    reason: "Property values have increased significantly this quarter.",
  };

  // Type-safe event listening
  // client.on('VoteCast', (event: VoteCastEvent) => {
  //   const votes = {
  //     0: 'Against',
  //     1: 'For',
  //     2: 'Abstain'
  //   };
  //   console.log(`${event.voter} voted ${votes[event.support]}`);
  //   console.log(`Voting power: ${event.votes}`);
  // });

  // client.on('ProposalExecuted', (event: ProposalExecutedEvent) => {
  //   console.log(`Proposal ${event.proposalId} executed!`);
  //   console.log(`Executed at block ${event.blockNumber}`);
  // });
}

// ============================================================================
// Example 4: Insurance - Create Policy & Submit Claims
// ============================================================================

import type {
  InsurancePolicy,
  InsuranceClaim,
  CreatePolicyParams,
  SubmitClaimParams,
  ApproveClaimParams,
  ClaimStatus,
  InsurancePolicyCreatedEvent,
  ClaimSubmittedEvent,
  ClaimApprovedEvent,
  ClaimPaidEvent,
} from "@propchain/sdk";

/**
 * Complete insurance workflow with claims processing
 */
async function insuranceWorkflow() {
  // 1. Create comprehensive insurance policy
  const policyParams: CreatePolicyParams = {
    propertyId: 42,
    coverageAmount: BigInt("1000000000000000000"), // 1M tokens
    premiumPerMonth: BigInt("10000000000000000"), // 0.01 tokens/month
    coverageType: "Comprehensive", // Fire, Theft, Liability, Comprehensive
    durationMonths: 12,
  };

  // 2. Submit insurance claim
  const claimParams: SubmitClaimParams = {
    policyId: 1,
    amount: BigInt("100000000000000000"), // 0.1M tokens claim
    description: "Roof damage from storm on 2024-04-15",
    evidence: [
      "ipfs://QmRoof1", // Photo evidence
      "ipfs://QmRoof2", // Inspector report
      "ipfs://QmRoof3", // Repair estimate
    ],
  };

  // 3. Approve claim (as insurance provider)
  const approveParams: ApproveClaimParams = {
    claimId: 1,
    approvedAmount: BigInt("95000000000000000"), // 95k approved
  };

  // Comprehensive event handling
  // client.on('ClaimSubmitted', (event: ClaimSubmittedEvent) => {
  //   console.log(`Claim #${event.claimId} submitted for $${event.claimAmount}`);
  //   console.log(`Policy #${event.policyId}`);
  // });

  // client.on('ClaimApproved', (event: ClaimApprovedEvent) => {
  //   const approvalRate = (event.approvedAmount / event.claimAmount) * 100;
  //   console.log(`Claim approved: ${approvalRate}% of requested amount`);
  //   console.log(`Approver: ${event.approvedBy}`);
  // });

  // client.on('ClaimPaid', (event: ClaimPaidEvent) => {
  //   console.log(`Payout sent: ${event.paidAmount} tokens`);
  //   console.log(`To: ${event.claimant}`);
  // });
}

// ============================================================================
// Example 5: Staking & Validation
// ============================================================================

import type {
  StakingPosition,
  StakingPool,
  ValidatorInfo,
  StakeParams,
  DelegateToValidatorParams,
  RegisterValidatorParams,
  StakedEvent,
  RewardsClaimedEvent,
  ValidatorRegisteredEvent,
} from "@propchain/sdk";

/**
 * Complete staking workflow with validator delegation
 */
async function stakingWorkflow() {
  // 1. Lock tokens for staking
  const stakeParams: StakeParams = {
    amount: BigInt("1000000000000000000"), // 1 token
    lockDurationDays: 365,
  };

  // 2. Delegate to validator
  const delegateParams: DelegateToValidatorParams = {
    validator: "1DHJdkTHc34W8sSZcgjccqQjQ9JVPCYb6kqxBz8VD5oW4XSA", // SS58 address
    amount: BigInt("500000000000000000"), // 0.5 token
  };

  // 3. Register as validator
  const validatorParams: RegisterValidatorParams = {
    commissionPercentage: 10, // Take 10% of delegated rewards
  };

  // Full event tracking
  // client.on('Staked', (event: StakedEvent) => {
  //   console.log(`Staker: ${event.staker}`);
  //   console.log(`Amount: ${event.amount}`);
  //   console.log(`New total staked: ${event.newTotal}`);
  // });

  // client.on('RewardsClaimed', (event: RewardsClaimedEvent) => {
  //   console.log(`Rewards earned: ${event.rewardAmount}`);
  //   console.log(`Claimed by: ${event.staker}`);
  // });

  // client.on('ValidatorRegistered', (event: ValidatorRegisteredEvent) => {
  //   console.log(`New validator: ${event.validator}`);
  //   console.log(`Commission: ${event.commissionPercentage}%`);
  // });
}

// ============================================================================
// Example 6: Fractional Ownership & Dividends
// ============================================================================

import type {
  FractionalOffering,
  Shareholder,
  DividendDistribution,
  CreateOfferingParams,
  BuySharesParams,
  ClaimDividendParams,
  SharesPurchasedEvent,
  DividendDistributedEvent,
  DividendClaimedEvent,
} from "@propchain/sdk";

/**
 * Complete fractional ownership workflow
 */
async function fractionalOwnershipWorkflow() {
  // 1. Create fractional offering
  const offeringParams = {
    propertyId: 42,
    totalShares: BigInt("1000000000000000000"), // 1B shares
    pricePerShare: BigInt("1000000000000"), // 0.000001 tokens
    minSharesPerBuy: BigInt("1000000000000"), // Minimum 1000 shares
    maxSharesPerBuyer: BigInt("100000000000000000"), // Max 100M per person
    offeringDurationDays: 30,
  };

  // 2. Purchase shares in offering
  const buyParams: BuySharesParams = {
    offeringId: 1,
    shareCount: BigInt("10000000000000"), // 10k shares
  };

  // 3. Claim dividend distribution
  const claimParams: ClaimDividendParams = {
    propertyId: 42,
    distributionId: 1,
  };

  // Event handling for fractional ownership
  // client.on('SharesPurchased', (event: SharesPurchasedEvent) => {
  //   const totalValue = event.totalCost;
  //   const costPerShare = totalValue / event.sharesPurchased;
  //   console.log(`Purchased ${event.sharesPurchased} shares`);
  //   console.log(`Cost per share: ${costPerShare}`);
  // });

  // client.on('DividendDistributed', (event: DividendDistributedEvent) => {
  //   const perShare = event.amountPerShare;
  //   console.log(`Dividend: ${perShare} tokens per share`);
  //   console.log(`Total distributed: ${event.totalAmount}`);
  //   console.log(`To ${event.recipientCount} shareholders`);
  // });
}

// ============================================================================
// Example 7: ZK Compliance & Privacy
// ============================================================================

import type {
  ZKProofSubmission,
  PrivacyPreferences,
  ComplianceCertificate,
  SubmitZKProofParams,
  UpdatePrivacyPreferencesParams,
  VerifyAddressOwnershipParams,
  ZKProofType,
  ZKProofSubmittedEvent,
  ZKProofVerifiedEvent,
  PrivacyPreferencesUpdatedEvent,
} from "@propchain/sdk";

/**
 * Complete ZK compliance workflow with privacy preservation
 */
async function zkComplianceWorkflow() {
  // 1. Submit age verification proof (privacy-preserving)
  const ageProofParams: SubmitZKProofParams = {
    proofType: "AgeVerification",
    proofData: "0x" + "a".repeat(128), // Encrypted proof
  };

  // 2. Verify income without disclosing actual amount
  const incomeProofParams: SubmitZKProofParams = {
    proofType: "IncomeVerification",
    proofData: "0x" + "b".repeat(128),
  };

  // 3. Configure privacy preferences
  const privacyParams: UpdatePrivacyPreferencesParams = {
    allowDataSharing: false,
    allowProofSharing: true,
    anonymizeTransactions: true,
    dataRetentionMonths: 6,
  };

  // 4. Verify address ownership
  const addressProofParams: VerifyAddressOwnershipParams = {
    proofData: "0x" + "c".repeat(128),
  };

  // Full privacy-aware event handling
  // client.on('ZKProofSubmitted', (event: ZKProofSubmittedEvent) => {
  //   console.log(`Proof ${event.proofType} submitted privately`);
  //   console.log(`Proof ID: ${event.proofId}`);
  // });

  // client.on('ZKProofVerified', (event: ZKProofVerifiedEvent) => {
  //   console.log(`Proof verification complete`);
  //   console.log(`Verified: ${event.verified}`);
  //   console.log(`Verifier: [REDACTED]`); // Privacy preserved
  // });

  // client.on('PrivacyPreferencesUpdated', (event: PrivacyPreferencesUpdatedEvent) => {
  //   console.log(`Privacy settings updated`);
  //   console.log(`Data sharing: ${event.allowDataSharing}`);
  //   console.log(`Proof sharing: ${event.allowProofSharing}`);
  // });
}

// ============================================================================
// Example 8: AI Valuation with Model Management
// ============================================================================

import type {
  ModelVersion,
  AIValuationResult,
  DriftDetectionResult,
  DeploymentStatus,
  DriftRecommendation,
  DeployModelParams,
  RequestValuationParams,
  DetectDriftParams,
} from "@propchain/sdk";

/**
 * Complete AI valuation workflow with model drift detection
 */
async function aiValuationWorkflow() {
  // 1. Deploy new ML model version
  const deployParams = {
    modelVersion: "v1.2.5",
    modelData: "0x" + "model_weights_binary_data",
  };

  // 2. Request property valuation
  const valuationParams: RequestValuationParams = {
    propertyId: 42,
    features: {
      location_latitude: 40.7128,
      location_longitude: -74.006,
      property_age_years: 15,
      size_sqm: 200,
      bedrooms: 3,
      bathrooms: 2,
      condition_score: 8.5,
      market_demand_index: 0.85,
    },
  };

  // 3. Detect and handle data drift
  const driftParams: DetectDriftParams = {
    modelVersion: "v1.2.5",
    windowSize: 1000, // Check last 1000 predictions
  };

  // Handle valuation results with full typing
  // const valuation = await aiClient.requestValuation(valuationParams);
  // console.log(`Predicted valuation: ${valuation.predictedValuation}`);
  // console.log(`Confidence: ${valuation.confidenceScore}%`);
  // console.log(`Price range: ${valuation.valuationRange[0]} - ${valuation.valuationRange[1]}`);
  // console.log(`Model: ${valuation.modelVersion}`);
  // console.log(`Expires: ${new Date(valuation.expiresAt * 1000)}`);

  // Handle drift detection with type safety
  // const driftResult = await aiClient.detectDrift(driftParams);
  // if (driftResult.driftDetected) {
  //   console.log(`Data drift detected: score ${driftResult.driftScore}`);
  //   console.log(`Recommendation: ${driftResult.recommendation}`);
  //   // Recommendations: Monitor, Retrain, Rollback, UpdateModel
  // }
}

// ============================================================================
// Example 9: Property Management & Maintenance
// ============================================================================

import type {
  ManagementAgreement,
  MaintenanceRequest,
  OccupancyStatus,
  CreateManagementAgreementParams,
  CreateMaintenanceRequestParams,
  UpdateOccupancyParams,
  CompleteMaintenanceParams,
  MaintenancePriority,
  MaintenanceStatus,
} from "@propchain/sdk";

/**
 * Complete property management workflow
 */
async function propertyManagementWorkflow() {
  // 1. Hire property manager
  const agreementParams: CreateManagementAgreementParams = {
    propertyId: 42,
    manager: "1DHJdkTHc34W8sSZcgjccqQjQ9JVPCYb6kqxBz8VD5oW4XSA",
    managementFeeBips: 200, // 2% management fee
    durationMonths: 12,
  };

  // 2. Create maintenance request
  const maintenanceParams: CreateMaintenanceRequestParams = {
    propertyId: 42,
    description: "HVAC system inspection and filter replacement",
    priority: "High",
    estimatedCost: BigInt("1000000000000000000"), // 1 token
  };

  // 3. Update occupancy information
  const occupancyParams: UpdateOccupancyParams = {
    propertyId: 42,
    isOccupied: true,
    tenant: "1DHJdkTHc34W8sSZcgjccqQjQ9JVPCYb6kqxBz8VD5oW4XSA",
    rentAmount: BigInt("100000000000000000"), // 0.1 tokens/month
  };

  // 4. Mark maintenance complete
  const completeParams: CompleteMaintenanceParams = {
    requestId: 1,
    actualCost: BigInt("950000000000000000"), // 0.95 tokens (under budget)
  };
}

// ============================================================================
// Example 10: Crowdfunding Campaign
// ============================================================================

import type {
  CrowdfundingCampaign,
  CrowdfundingContribution,
  CampaignMilestone,
  CreateCampaignParams,
  ContributeParams,
  AddMilestoneParams,
  ReleaseMilestoneParams,
  CampaignStatus,
  ContributionMadeEvent,
  CampaignFundedEvent,
} from "@propchain/sdk";

/**
 * Complete crowdfunding workflow with milestone releases
 */
async function crowdfundingWorkflow() {
  // 1. Create crowdfunding campaign
  const campaignParams: CreateCampaignParams = {
    propertyId: 42,
    targetAmount: BigInt("10000000000000000000"), // 10 tokens target
    deadlineUnix: Math.floor(Date.now() / 1000) + 30 * 24 * 60 * 60, // 30 days
    minContribution: BigInt("10000000000000"), // 0.00001 tokens minimum
    maxContributorsPerProperty: 1000,
  };

  // 2. Make contribution
  const contributionParams: ContributeParams = {
    campaignId: 1,
    amount: BigInt("100000000000000000"), // 0.1 tokens
  };

  // 3. Add milestone for disbursement
  const milestoneParams: AddMilestoneParams = {
    campaignId: 1,
    title: "Foundation & Structure Complete",
    description: "Property foundation laid and structure erected",
    targetAmount: BigInt("5000000000000000000"), // 5 tokens for this phase
    dueDateUnix: Math.floor(Date.now() / 1000) + 60 * 24 * 60 * 60,
  };

  // 4. Release milestone funds
  const releaseParams: ReleaseMilestoneParams = {
    milestoneId: 1,
  };

  // Track campaign funding
  // client.on('ContributionMade', (event: ContributionMadeEvent) => {
  //   console.log(`New contribution: ${event.amount}`);
  //   console.log(`Campaign total: ${event.totalRaised}`);
  //   console.log(`${((event.totalRaised / targetAmount) * 100).toFixed(1)}% funded`);
  // });

  // client.on('CampaignFunded', (event: CampaignFundedEvent) => {
  //   console.log(`Campaign fully funded! Total: ${event.totalRaised}`);
  //   console.log(`Funded at block ${event.blockNumber}`);
  // });
}

// ============================================================================
// Example 11: Prediction Market for Property Prices
// ============================================================================

import type {
  PredictionMarket,
  PredictionOutcome,
  PredictionPosition,
  CreateMarketParams,
  BetOnOutcomeParams,
  ResolveMarketParams,
  ClaimWinningsParams,
  MarketStatus,
  PredictionBetPlacedEvent,
  MarketResolvedEvent,
  WinningsClaimedEvent,
} from "@propchain/sdk";

/**
 * Complete prediction market workflow
 */
async function predictionMarketWorkflow() {
  // 1. Create property price prediction market
  const marketParams = {
    propertyId: 42,
    question: "Will Property #42 valuation exceed 100 tokens by end of 2024?",
    description: "Prediction market for property price appreciation",
    outcomes: ["Yes (>100 tokens)", "No (<=100 tokens)"],
    resolutionDateUnix: Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60,
  };

  // 2. Place bet on "Yes" outcome
  const betParams: BetOnOutcomeParams = {
    marketId: 1,
    outcomeId: 0, // "Yes" option
    amount: BigInt("1000000000000000000"), // 1 token bet
  };

  // 3. When market resolves, claim winnings
  const winningsParams: ClaimWinningsParams = {
    marketId: 1,
    outcomeIds: [0], // Won on this outcome
  };

  // Track market activity
  // client.on('PredictionBetPlaced', (event: PredictionBetPlacedEvent) => {
  //   const returnOnBet = (event.shares / event.amount).toFixed(2);
  //   console.log(`Bet placed: ${event.amount} tokens = ${event.shares} shares`);
  //   console.log(`Implied odds: 1:${returnOnBet}`);
  // });

  // client.on('MarketResolved', (event: MarketResolvedEvent) => {
  //   console.log(`Market resolved!`);
  //   console.log(`Winning outcome: ${event.winningOutcomeId}`);
  //   console.log(`Resolution: ${event.resolution}`);
  // });

  // client.on('WinningsClaimed', (event: WinningsClaimedEvent) => {
  //   console.log(`Winnings paid: ${event.winnings} tokens`);
  // });
}

// ============================================================================
// Example 12: Complete Multi-Contract Interaction
// ============================================================================

/**
 * Complex real-world scenario: Property Investment with Insurance
 *
 * Flow:
 * 1. User buys fractional shares via DEX
 * 2. Buys insurance policy for the property
 * 3. Deposits stablecoins in lending pool to earn interest
 * 4. Stakes governance token to vote on property improvements
 * 5. Receives ZK compliance verification
 */
async function complexRealWorldScenario() {
  // Step 1: Acquire fractional shares via DEX
  console.log("Step 1: Acquiring fractional shares...");
  // const buySharesSwap = {
  //   pairId: 1,
  //   isBuyOrder: true,
  //   amountIn: BigInt('50000000000000000'),
  //   minAmountOut: BigInt('100000000000000'),
  // };

  // Step 2: Get insurance for property
  console.log("Step 2: Purchasing insurance...");
  // const insurancePolicy = {
  //   propertyId: 42,
  //   coverageAmount: BigInt('1000000000000000000'),
  //   premiumPerMonth: BigInt('10000000000000000'),
  //   coverageType: 'Comprehensive',
  //   durationMonths: 12,
  // };

  // Step 3: Deposit into lending pool
  console.log("Step 3: Depositing for yield...");
  // const deposit = {
  //   poolId: 1,
  //   amount: BigInt('100000000000000000000'),
  // };

  // Step 4: Stake governance tokens
  console.log("Step 4: Staking for governance...");
  // const stake = {
  //   amount: BigInt('1000000000000000000'),
  //   lockDurationDays: 365,
  // };

  // Step 5: Submit ZK proof for compliance
  console.log("Step 5: Submitting compliance proof...");
  // const zkProof = {
  //   proofType: 'AccreditedInvestor',
  //   proofData: '0x...',
  // };

  console.log("Investment portfolio setup complete!");
  console.log("- Owns fractional shares of property");
  console.log("- Has insurance protection");
  console.log("- Earning interest from deposits");
  console.log("- Participating in governance");
  console.log("- Privacy-preserved compliance verified");
}

// ============================================================================
// Type Safety Benefits Demonstrated
// ============================================================================

/**
 * Example showing TypeScript compile-time safety
 */
export function typeCheckingDemo() {
  // ✅ Correct: Proper typing
  const correctAmount: bigint = BigInt("1000000000000000000");

  // ❌ Compile error: string instead of bigint
  // const wrongAmount: bigint = '1000000000000000000';

  // ✅ Correct: Valid enum value
  const correctStatus = "Active" as const;

  // ❌ Compile error: Invalid enum value
  // const wrongStatus = 'NotAStatus';

  // ✅ Correct: All required params
  const correctCall: CreatePoolParams = {
    baseToken: 1,
    quoteToken: 2,
    baseReserve: BigInt("1000"),
    quoteReserve: BigInt("500"),
    feePercentage: 30,
  };

  // ❌ Compile error: Missing required 'feePercentage'
  // const incompleteCall: CreatePoolParams = {
  //   baseToken: 1,
  //   quoteToken: 2,
  //   baseReserve: BigInt('1000'),
  //   quoteReserve: BigInt('500'),
  // };
}

// ============================================================================
// Summary
// ============================================================================

/**
 * This file demonstrates:
 *
 * ✅ Type-safe contract interactions
 * ✅ Proper use of bigint for amounts
 * ✅ Enum validation at compile-time
 * ✅ Complete event typing
 * ✅ Multi-contract workflows
 * ✅ Error handling patterns
 * ✅ Real-world usage scenarios
 *
 * All code examples use the comprehensive TypeScript types from:
 * - types/contracts.ts (domain types)
 * - types/contract-events.ts (event types)
 * - types/contract-calls.ts (parameter & return types)
 */
