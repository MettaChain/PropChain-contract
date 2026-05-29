/**
 * @propchain/sdk — Signer Utilities
 *
 * Helpers for managing account signers: browser extension integration,
 * keyring pairs for testing, and account selection.
 *
 * @module utils/signer
 */

import { Keyring } from '@polkadot/keyring';
import type { KeyringPair } from '@polkadot/keyring/types';
import type { InjectedAccountWithMeta } from '@polkadot/extension-inject/types';

// ============================================================================
// Browser Extension Integration
// ============================================================================

/**
 * Connects to the Polkadot.js browser extension and retrieves available accounts.
 *
 * @param appName - The application name shown in the extension approval dialogue
 * @returns An array of injected accounts
 * @throws Error if the extension is not installed or the user denies access
 *
 * @example
 * ```typescript
 * const accounts = await connectExtension('PropChain dApp');
 * console.log('Available accounts:', accounts.map(a => a.address));
 * ```
 */
export async function connectExtension(
  appName: string = 'PropChain',
): Promise<InjectedAccountWithMeta[]> {
  // Dynamic import to avoid SSR issues
  const { web3Enable, web3Accounts } = await import('@polkadot/extension-dapp');

  const extensions = await web3Enable(appName);
  if (extensions.length === 0) {
    throw new Error(
      'No Polkadot.js extension found. Please install it from https://polkadot.js.org/extension/',
    );
  }

  const accounts = await web3Accounts();
  return accounts;
}

/**
 * Gets the signer for a specific account from the browser extension.
 *
 * @param address - The SS58 account address to get the signer for
 * @returns The injected signer interface
 * @throws Error if the extension is not available or the account is not found
 *
 * @example
 * ```typescript
 * const signer = await getExtensionSigner('5GrwvaEF...');
 * // Use with api.tx
 * await api.tx.someModule.someMethod().signAndSend(address, { signer });
 * ```
 */
export async function getExtensionSigner(address: string) {
  const { web3FromAddress } = await import('@polkadot/extension-dapp');
  const injector = await web3FromAddress(address);
  return injector.signer;
}

// ============================================================================
// Keyring (Programmatic Signing)
// ============================================================================

/**
 * Creates a keyring pair from a seed phrase or URI.
 *
 * **Warning:** Use only for development/testing. Never use mnemonics in
 * production frontend code.
 *
 * @param seed - Seed phrase, URI (e.g. `//Alice`), or hex seed
 * @param keyringType - Key type: 'sr25519' (default) or 'ed25519'
 * @returns A `KeyringPair` for signing transactions
 *
 * @example
 * ```typescript
 * // Development account
 * const alice = createKeyringPair('//Alice');
 *
 * // From mnemonic (testing only!)
 * const pair = createKeyringPair('word1 word2 ... word12');
 * ```
 */
export function createKeyringPair(
  seed: string,
  keyringType: 'sr25519' | 'ed25519' = 'sr25519',
): KeyringPair {
  const keyring = new Keyring({ type: keyringType, ss58Format: 42 });
  return keyring.addFromUri(seed);
}

/**
 * Creates standard development keyring pairs (Alice, Bob, Charlie, Dave, Eve, Ferdie).
 *
 * @returns Object with named keyring pairs
 *
 * @example
 * ```typescript
 * const { alice, bob } = createDevAccounts();
 * console.log('Alice:', alice.address);
 * ```
 */
export function createDevAccounts(): Record<string, KeyringPair> {
  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  return {
    alice: keyring.addFromUri('//Alice'),
    bob: keyring.addFromUri('//Bob'),
    charlie: keyring.addFromUri('//Charlie'),
    dave: keyring.addFromUri('//Dave'),
    eve: keyring.addFromUri('//Eve'),
    ferdie: keyring.addFromUri('//Ferdie'),
  };
}
