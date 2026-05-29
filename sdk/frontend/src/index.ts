/**
 * @propchain/sdk
 *
 * Comprehensive TypeScript SDK for PropChain smart contract integration.
 * Supports property tokenization, escrow, oracle, governance, and
 * cross-chain bridge operations on Substrate/Polkadot.
 *
 * @packageDocumentation
 */

// ============================================================================
// Types
// ============================================================================
export type {
  // Core property types
  PropertyMetadata,
  PropertyInfo,
  PortfolioSummary,
  PortfolioDetails,
  PortfolioProperty,
  // Escrow types
  EscrowInfo,
  // Oracle / valuation types
  PropertyValuation,
  ValuationWithConfidence,
  VolatilityMetrics,
  PriceData,
  OracleSource,
  PriceAlert,
  ComparableProperty,
  LocationAdjustment,
  MarketTrend,
  // Badge types
  Badge,
  VerificationRequest,
  Appeal,
  // Bridge types
  BridgeStatus,
  BridgeMonitoringInfo,
  BridgeTransaction,
  MultisigBridgeRequest,
  BridgeConfig,
  ChainBridgeInfo,
  // Token types
  OwnershipTransfer,
  ComplianceInfo,
  DocumentInfo,
  Proposal,
  Ask,
  TaxRecord,
  // Health / monitoring types
  HealthStatus,
  GasMetrics,
  GlobalAnalytics,
  BatchConfig,
  BatchResult,
  BatchItemFailure,
  BatchMetrics,
  BatchOperationStats,
  // Pause types
  PauseInfo,
  // Fractional types
  FractionalInfo,
  // SDK types
  ClientOptions,
  ContractAddresses,
  TxResult,
  TxStatusUpdate,
  TxProgressCallback,
  ContractEvent,
  GasEstimation,
  NetworkConfig,
  Subscription,
} from './types';

export {
  // Enums
  PropertyType,
  ApprovalType,
  ValuationMethod,
  OracleSourceType,
  BadgeType,
  VerificationStatus,
  AppealStatus,
  BridgeOperationStatus,
  RecoveryAction,
  FeeOperation,
  ProposalStatus,
  // Error enums
  PropertyRegistryError,
  PropertyTokenError,
  OracleErrorCode,
  // Transaction progress
  TxProgressStatus,
} from './types';

// ============================================================================
// Event Types
// ============================================================================
export type {
  PropChainEventName,
  PropChainEventMap,
  // Individual event types
  ContractInitializedEvent,
  PropertyRegisteredEvent,
  PropertyTransferredEvent,
  PropertyMetadataUpdatedEvent,
  ApprovalGrantedEvent,
  ApprovalClearedEvent,
  EscrowCreatedEvent,
  EscrowReleasedEvent,
  EscrowRefundedEvent,
  AdminChangedEvent,
  BatchPropertyRegisteredEvent,
  BatchPropertyTransferredEvent,
  BatchMetadataUpdatedEvent,
  BatchOperationCompletedEvent,
  BadgeIssuedEvent,
  BadgeRevokedEvent,
  VerificationRequestedEvent,
  VerificationReviewedEvent,
  TransferEvent,
  ApprovalEvent,
  ApprovalForAllEvent,
  PropertyTokenMintedEvent,
  LegalDocumentAttachedEvent,
  ComplianceVerifiedEvent,
  TokenBridgedEvent,
  BridgeRequestCreatedEvent,
  BridgeRequestSignedEvent,
  BridgeExecutedEvent,
  BridgeFailedEvent,
  SharesIssuedEvent,
  SharesRedeemedEvent,
  DividendsDepositedEvent,
  DividendsWithdrawnEvent,
  ProposalCreatedEvent,
  VotedEvent,
  ProposalExecutedEvent,
  AskPlacedEvent,
  AskCancelledEvent,
  SharesPurchasedEvent,
  PropertyManagementContractSetEvent,
  ManagementAgentAssignedEvent,
  ManagementAgentClearedEvent,
} from './types/events';

// ============================================================================
// Clients
// ============================================================================
export { PropChainClient } from './client/PropChainClient';
export { FederatedPropChainClient } from './client/FederatedPropChainClient';
export { PropertyRegistryClient } from './client/PropertyRegistryClient';
export { PropertyTokenClient } from './client/PropertyTokenClient';
export { EscrowClient } from './client/EscrowClient';
export { OracleClient } from './client/OracleClient';

// ============================================================================
// Contract Module Federation (Dynamic Loading)
// ============================================================================
export type { ContractModule, CreateContractClientArgs } from './modules/types';
export type { BuiltInContractModuleId } from './modules/builtin';
export {
  loadContractModule,
  registerContractModule,
  listContractModules,
} from './modules/loader';

// ============================================================================
// Utilities
// ============================================================================
export {
  createApi,
  connectWithRetry,
  connectToNetwork,
  getNetworkConfig,
  NETWORKS,
} from './utils/connection';

export {
  connectExtension,
  getExtensionSigner,
  createKeyringPair,
  createDevAccounts,
} from './utils/signer';

export {
  formatBalance,
  parseBalance,
  formatValuation,
  formatAddress,
  truncateAddress,
  formatTimestamp,
  relativeTime,
  formatNumber,
  formatPropertySize,
} from './utils/formatters';

export {
  PropChainError,
  ConnectionError,
  TransactionError,
  ErrorCategory,
  decodeContractError,
  isContractRevert,
  getUserFriendlyMessage,
} from './utils/errors';

export {
  decodeEvent,
  decodeTransactionEvents,
  subscribeToEvents,
  subscribeToNamedEvent,
  filterEvents,
  extractTypedEvents,
} from './utils/events';
