/**
 * Type Validation Tests
 *
 * Verifies that all TypeScript types compile correctly, match expected
 * shapes, and can be instantiated without runtime errors.
 */

import { describe, it, expect } from 'vitest';
import type {
  PropertyMetadata,
  PropertyInfo,
  EscrowInfo,
  HealthStatus,
  GlobalAnalytics,
  Badge,
  VerificationRequest,
  Appeal,
  BridgeStatus,
  BridgeMonitoringInfo,
  BridgeTransaction,
  MultisigBridgeRequest,
  PortfolioSummary,
  PortfolioDetails,
  BatchResult,
  BatchConfig,
  Proposal,
  Ask,
  TaxRecord,
  OwnershipTransfer,
  ComplianceInfo,
  DocumentInfo,
  PauseInfo,
  FractionalInfo,
  GasMetrics,
  TxResult,
  ContractEvent,
  ClientOptions,
  ContractAddresses,
  GasEstimation,
  NetworkConfig,
  Subscription,
} from '../src/types';

import {
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
  PropertyRegistryError,
  PropertyTokenError,
  OracleErrorCode,
} from '../src/types';

describe('Type Definitions', () => {
  describe('Core Property Types', () => {
    it('should create a valid PropertyMetadata', () => {
      const metadata: PropertyMetadata = {
        location: '123 Main St, New York, NY',
        size: 2500,
        legalDescription: 'Lot 1, Block 2, City Subdivision',
        valuation: BigInt('50000000000000'),
        documentsUrl: 'ipfs://QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco',
      };

      expect(metadata.location).toBe('123 Main St, New York, NY');
      expect(metadata.size).toBe(2500);
      expect(metadata.valuation).toBe(BigInt('50000000000000'));
    });

    it('should create a valid PropertyInfo', () => {
      const info: PropertyInfo = {
        id: 1,
        owner: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        metadata: {
          location: '456 Oak Ave',
          size: 3500,
          legalDescription: 'Lot 5, Block 3',
          valuation: BigInt('100000000000000'),
          documentsUrl: 'ipfs://Qm...',
        },
        registeredAt: 1700000000,
      };

      expect(info.id).toBe(1);
      expect(info.owner).toContain('5Grwva');
    });

    it('should have correct PropertyType enum values', () => {
      expect(PropertyType.Residential).toBe('Residential');
      expect(PropertyType.Commercial).toBe('Commercial');
      expect(PropertyType.Industrial).toBe('Industrial');
      expect(PropertyType.Land).toBe('Land');
      expect(PropertyType.MultiFamily).toBe('MultiFamily');
      expect(PropertyType.Retail).toBe('Retail');
      expect(PropertyType.Office).toBe('Office');
    });
  });

  describe('Escrow Types', () => {
    it('should create a valid EscrowInfo', () => {
      const escrow: EscrowInfo = {
        id: 1,
        propertyId: 42,
        buyer: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
        seller: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        amount: BigInt('500000000000000'),
        released: false,
      };

      expect(escrow.id).toBe(1);
      expect(escrow.released).toBe(false);
    });

    it('should have correct ApprovalType enum', () => {
      expect(ApprovalType.Release).toBe('Release');
      expect(ApprovalType.Refund).toBe('Refund');
      expect(ApprovalType.EmergencyOverride).toBe('EmergencyOverride');
    });
  });

  describe('Oracle Types', () => {
    it('should have correct ValuationMethod enum', () => {
      expect(ValuationMethod.Automated).toBe('Automated');
      expect(ValuationMethod.AIValuation).toBe('AIValuation');
      expect(ValuationMethod.MarketData).toBe('MarketData');
    });

    it('should have correct OracleSourceType enum', () => {
      expect(OracleSourceType.Chainlink).toBe('Chainlink');
      expect(OracleSourceType.AIModel).toBe('AIModel');
    });
  });

  describe('Badge Types', () => {
    it('should have correct BadgeType enum', () => {
      expect(BadgeType.OwnerVerification).toBe('OwnerVerification');
      expect(BadgeType.DocumentVerification).toBe('DocumentVerification');
      expect(BadgeType.LegalCompliance).toBe('LegalCompliance');
      expect(BadgeType.PremiumListing).toBe('PremiumListing');
    });

    it('should create a valid Badge', () => {
      const badge: Badge = {
        badgeType: BadgeType.OwnerVerification,
        issuedAt: 1700000000,
        issuedBy: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        expiresAt: 1731536000,
        metadataUrl: 'https://verify.propchain.io/badge/1',
        revoked: false,
        revokedAt: null,
        revocationReason: '',
      };

      expect(badge.badgeType).toBe(BadgeType.OwnerVerification);
      expect(badge.revoked).toBe(false);
      expect(badge.expiresAt).toBe(1731536000);
    });
  });

  describe('Bridge Types', () => {
    it('should have all BridgeOperationStatus values', () => {
      expect(BridgeOperationStatus.None).toBe('None');
      expect(BridgeOperationStatus.Pending).toBe('Pending');
      expect(BridgeOperationStatus.Locked).toBe('Locked');
      expect(BridgeOperationStatus.InTransit).toBe('InTransit');
      expect(BridgeOperationStatus.Completed).toBe('Completed');
      expect(BridgeOperationStatus.Failed).toBe('Failed');
      expect(BridgeOperationStatus.Recovering).toBe('Recovering');
      expect(BridgeOperationStatus.Expired).toBe('Expired');
    });

    it('should have correct RecoveryAction enum', () => {
      expect(RecoveryAction.UnlockToken).toBe('UnlockToken');
      expect(RecoveryAction.RetryBridge).toBe('RetryBridge');
      expect(RecoveryAction.CancelBridge).toBe('CancelBridge');
    });
  });

  describe('Governance Types', () => {
    it('should have correct ProposalStatus enum', () => {
      expect(ProposalStatus.Open).toBe('Open');
      expect(ProposalStatus.Executed).toBe('Executed');
      expect(ProposalStatus.Rejected).toBe('Rejected');
      expect(ProposalStatus.Closed).toBe('Closed');
    });

    it('should create a valid Proposal', () => {
      const proposal: Proposal = {
        id: 1,
        tokenId: 42,
        descriptionHash: '0xabcdef',
        quorum: BigInt('1000'),
        forVotes: BigInt('600'),
        againstVotes: BigInt('100'),
        status: ProposalStatus.Open,
        createdAt: 1700000000,
      };

      expect(proposal.forVotes > proposal.againstVotes).toBe(true);
    });
  });

  describe('Error Types', () => {
    it('should have all PropertyRegistryError variants', () => {
      expect(PropertyRegistryError.PropertyNotFound).toBe('PropertyNotFound');
      expect(PropertyRegistryError.Unauthorized).toBe('Unauthorized');
      expect(PropertyRegistryError.ContractPaused).toBe('ContractPaused');
      expect(PropertyRegistryError.BatchSizeExceeded).toBe('BatchSizeExceeded');
    });

    it('should have all PropertyTokenError variants', () => {
      expect(PropertyTokenError.TokenNotFound).toBe('TokenNotFound');
      expect(PropertyTokenError.BridgeLocked).toBe('BridgeLocked');
      expect(PropertyTokenError.InsufficientBalance).toBe('InsufficientBalance');
    });

    it('should have all OracleErrorCode variants', () => {
      expect(OracleErrorCode.PropertyNotFound).toBe('PropertyNotFound');
      expect(OracleErrorCode.InsufficientSources).toBe('InsufficientSources');
      expect(OracleErrorCode.PriceFeedError).toBe('PriceFeedError');
    });
  });

  describe('SDK Types', () => {
    it('should create valid ClientOptions', () => {
      const options: ClientOptions = {
        autoReconnect: true,
        maxReconnectAttempts: 3,
        connectionTimeout: 15000,
      };

      expect(options.autoReconnect).toBe(true);
    });

    it('should create valid ContractAddresses', () => {
      const addresses: ContractAddresses = {
        propertyRegistry: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        propertyToken: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
      };

      expect(addresses.propertyRegistry).toBeDefined();
      expect(addresses.oracle).toBeUndefined();
    });

    it('should create valid TxResult', () => {
      const result: TxResult = {
        txHash: '0xabc123',
        blockHash: '0xdef456',
        blockNumber: 100,
        events: [{ name: 'PropertyRegistered', args: { propertyId: 1 } }],
        success: true,
      };

      expect(result.success).toBe(true);
      expect(result.events).toHaveLength(1);
    });

    it('should create valid HealthStatus', () => {
      const health: HealthStatus = {
        isHealthy: true,
        isPaused: false,
        contractVersion: 1,
        propertyCount: 42,
        escrowCount: 5,
        hasOracle: true,
        hasComplianceRegistry: true,
        hasFeeManager: false,
        blockNumber: 1000,
        timestamp: 1700000000,
      };

      expect(health.isHealthy).toBe(true);
      expect(health.propertyCount).toBe(42);
    });
  });

  describe('Fee Types', () => {
    it('should have correct FeeOperation enum', () => {
      expect(FeeOperation.RegisterProperty).toBe('RegisterProperty');
      expect(FeeOperation.TransferProperty).toBe('TransferProperty');
      expect(FeeOperation.CreateEscrow).toBe('CreateEscrow');
    });
  });
});
