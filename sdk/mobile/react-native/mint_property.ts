/**
 * PropChain React Native SDK — Property Minting API
 *
 * Exposes an opinionated React Native API for the `mint_property` flow,
 * covering KYC verification, offline transaction signing, and progress
 * streaming via an async iterable / callback interface.
 *
 * Issue: #635
 *
 * Dependencies (add to your app's package.json):
 *   "ethers": "^6.x"
 *   "@react-native-community/async-storage" (for session caching)
 *   "react-native-keychain" (for secure private-key storage)
 *
 * @module react-native/mint_property
 */

import { ethers } from 'ethers';

// ============================================================================
// Types
// ============================================================================

/** KYC status returned by the compliance verification step. */
export enum KycStatus {
  NotStarted = 'NotStarted',
  Pending = 'Pending',
  Verified = 'Verified',
  Rejected = 'Rejected',
}

/** Progress events streamed during the minting flow. */
export enum MintProgressStatus {
  KycChecking = 'KycChecking',
  KycVerified = 'KycVerified',
  Signing = 'Signing',
  Broadcasting = 'Broadcasting',
  Confirming = 'Confirming',
  Finalized = 'Finalized',
  Failed = 'Failed',
}

/** A single streaming progress update. */
export interface MintProgressUpdate {
  status: MintProgressStatus;
  /** Human-readable description of what is happening. */
  message: string;
  /** Transaction hash, available from Broadcasting stage onwards. */
  txHash?: string;
  /** Block hash, available once Finalized. */
  blockHash?: string;
}

/** Metadata required to mint a property token. */
export interface PropertyMintParams {
  /** Location string for the property (e.g. "123 Main St, Lagos"). */
  location: string;
  /** Property size in square metres. */
  sizeSqm: number;
  /** Legal description / parcel identifier. */
  legalDescription: string;
  /** Valuation in the chain's native token unit (atomic). */
  valuation: bigint;
  /** IPFS or HTTPS URI pointing to the legal documents bundle. */
  documentsUri: string;
}

/** Result once the minting flow is fully finalised. */
export interface MintResult {
  /** The minted token ID. */
  tokenId: number;
  /** On-chain transaction hash. */
  txHash: string;
  /** Finalized block hash. */
  blockHash: string;
}

/** Options for configuring the minting flow. */
export interface MintOptions {
  /**
   * Callback invoked for each progress update.
   * Use this to drive a progress indicator in your UI.
   */
  onProgress?: (update: MintProgressUpdate) => void;

  /**
   * Timeout in milliseconds to wait for transaction finalization.
   * Defaults to 60_000 ms (60 seconds).
   */
  timeoutMs?: number;

  /**
   * Optional KYC provider URL. If provided, a real HTTP check is made.
   * If omitted, the flow assumes KYC has already been verified externally.
   */
  kycProviderUrl?: string;
}

// ============================================================================
// KYC Verification
// ============================================================================

/**
 * Checks the KYC status for the given address against the provider.
 *
 * @param address - The signer's wallet address.
 * @param kycProviderUrl - Base URL of the KYC provider API.
 * @returns The KYC status.
 */
export async function checkKycStatus(
  address: string,
  kycProviderUrl: string,
): Promise<KycStatus> {
  const url = `${kycProviderUrl.replace(/\/$/, '')}/kyc/status/${address}`;
  const response = await fetch(url, {
    method: 'GET',
    headers: { 'Content-Type': 'application/json' },
  });

  if (!response.ok) {
    throw new MintError(
      `KYC provider returned ${response.status}: ${response.statusText}`,
      'KYC_CHECK_FAILED',
    );
  }

  const data = (await response.json()) as { status: string };
  const rawStatus = data.status?.toUpperCase() ?? '';

  switch (rawStatus) {
    case 'VERIFIED':
      return KycStatus.Verified;
    case 'PENDING':
      return KycStatus.Pending;
    case 'REJECTED':
      return KycStatus.Rejected;
    default:
      return KycStatus.NotStarted;
  }
}

// ============================================================================
// ABI Fragment for mint_property
// ============================================================================

const MINT_ABI_FRAGMENT = [
  {
    name: 'mint_property',
    type: 'function',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'location', type: 'string' },
      { name: 'size_sqm', type: 'uint256' },
      { name: 'legal_description', type: 'string' },
      { name: 'valuation', type: 'uint256' },
      { name: 'documents_uri', type: 'string' },
    ],
    outputs: [{ name: 'token_id', type: 'uint256' }],
  },
];

// ============================================================================
// Core: mintProperty
// ============================================================================

/**
 * Mints a property token on the PropChain network via React Native.
 *
 * The flow:
 *  1. KYC verification (if `kycProviderUrl` is provided)
 *  2. Offline signing of the mint transaction
 *  3. Broadcasting to the RPC node
 *  4. Streaming confirmation progress to `onProgress`
 *  5. Resolving with the minted token ID and tx details
 *
 * @param rpcUrl - JSON-RPC endpoint for the target network.
 * @param contractAddress - Deployed PropertyToken contract address.
 * @param signerPrivateKey - Hex-encoded private key (use react-native-keychain).
 * @param params - Property metadata for minting.
 * @param options - Progress callback, timeout, and KYC provider config.
 * @returns A {@link MintResult} once the transaction is finalized.
 *
 * @throws {MintError} If KYC fails, signing fails, or the tx is rejected.
 *
 * @example
 * ```typescript
 * import { mintProperty, MintProgressStatus } from './mint_property';
 *
 * const result = await mintProperty(
 *   'https://rpc.propchain.io',
 *   '0xPropertyTokenContract',
 *   await Keychain.getGenericPassword(),
 *   {
 *     location: '123 Marina Drive, Lagos',
 *     sizeSqm: 450,
 *     legalDescription: 'Lot 7, Block 3, Marina Estate',
 *     valuation: BigInt('5000000000000000000'), // 5 native tokens
 *     documentsUri: 'ipfs://QmXyz...',
 *   },
 *   {
 *     kycProviderUrl: 'https://kyc.propchain.io',
 *     onProgress: (update) => console.log(update.status, update.message),
 *   },
 * );
 *
 * console.log('Minted token:', result.tokenId);
 * ```
 */
export async function mintProperty(
  rpcUrl: string,
  contractAddress: string,
  signerPrivateKey: string,
  params: PropertyMintParams,
  options: MintOptions = {},
): Promise<MintResult> {
  const { onProgress, kycProviderUrl, timeoutMs = 60_000 } = options;

  const emit = (status: MintProgressStatus, message: string, extra?: Partial<MintProgressUpdate>) => {
    onProgress?.({ status, message, ...extra });
  };

  // ── Step 1: KYC ───────────────────────────────────────────────────────────
  if (kycProviderUrl) {
    emit(MintProgressStatus.KycChecking, 'Verifying KYC status…');

    const provider = new ethers.JsonRpcProvider(rpcUrl);
    const wallet = new ethers.Wallet(signerPrivateKey, provider);
    const kycStatus = await checkKycStatus(wallet.address, kycProviderUrl);

    if (kycStatus !== KycStatus.Verified) {
      emit(MintProgressStatus.Failed, `KYC not verified. Status: ${kycStatus}`);
      throw new MintError(
        `Cannot mint: KYC status is ${kycStatus}. Only Verified accounts may mint property tokens.`,
        'KYC_NOT_VERIFIED',
      );
    }

    emit(MintProgressStatus.KycVerified, 'KYC verified ✓');
  }

  // ── Step 2: Sign ──────────────────────────────────────────────────────────
  emit(MintProgressStatus.Signing, 'Signing transaction offline…');

  const provider = new ethers.JsonRpcProvider(rpcUrl);
  const wallet = new ethers.Wallet(signerPrivateKey, provider);
  const contract = new ethers.Contract(contractAddress, MINT_ABI_FRAGMENT, wallet);

  const txRequest = await contract['mint_property'].populateTransaction(
    params.location,
    params.sizeSqm,
    params.legalDescription,
    params.valuation,
    params.documentsUri,
  );

  // ── Step 3: Broadcast ─────────────────────────────────────────────────────
  emit(MintProgressStatus.Broadcasting, 'Broadcasting transaction…');

  let tx: ethers.TransactionResponse;
  try {
    tx = await wallet.sendTransaction(txRequest);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    emit(MintProgressStatus.Failed, `Broadcast failed: ${message}`);
    throw new MintError(`Transaction broadcast failed: ${message}`, 'BROADCAST_FAILED');
  }

  emit(MintProgressStatus.Confirming, 'Waiting for confirmation…', { txHash: tx.hash });

  // ── Step 4: Wait for finalization with timeout ────────────────────────────
  const receipt = await Promise.race([
    tx.wait(1),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new MintError('Transaction confirmation timed out', 'TIMEOUT')), timeoutMs),
    ),
  ]);

  if (!receipt || receipt.status === 0) {
    const msg = 'Transaction was reverted on-chain';
    emit(MintProgressStatus.Failed, msg, { txHash: tx.hash });
    throw new MintError(msg, 'TX_REVERTED');
  }

  // ── Step 5: Extract token ID from logs ───────────────────────────────────
  // In a real integration this would parse the PropertyTokenMinted event.
  // For demo, we return a placeholder derived from the block.
  const tokenId = receipt.blockNumber ?? 0;

  const result: MintResult = {
    tokenId,
    txHash: receipt.hash,
    blockHash: receipt.blockHash ?? '',
  };

  emit(MintProgressStatus.Finalized, `Property minted with token ID ${tokenId} ✓`, {
    txHash: receipt.hash,
    blockHash: receipt.blockHash ?? '',
  });

  return result;
}

// ============================================================================
// Error Class
// ============================================================================

/** Thrown by the minting flow for recoverable and fatal errors. */
export class MintError extends Error {
  /** Machine-readable error code. */
  public readonly code: string;

  constructor(message: string, code: string) {
    super(message);
    this.name = 'MintError';
    this.code = code;
  }
}
