import React from 'react';

interface ConnectWalletProps {
  connected: boolean;
  account: string | null;
  onConnect: (address: string) => void;
  onDisconnect: () => void;
}

/**
 * Wallet connection component.
 *
 * In a real app, this would use the PropChain SDK's `connectExtension()`
 * to interact with the Polkadot.js browser extension.
 *
 * @example
 * ```typescript
 * import { connectExtension } from '@propchain/sdk';
 *
 * const accounts = await connectExtension('PropChain dApp');
 * const selectedAccount = accounts[0];
 * ```
 */
export function ConnectWallet({
  connected,
  account,
  onConnect,
  onDisconnect,
}: ConnectWalletProps) {
  const handleConnect = async () => {
    // In production, use:
    // const { connectExtension } = await import('@propchain/sdk');
    // const accounts = await connectExtension('PropChain dApp');
    // onConnect(accounts[0].address);

    // For demo, simulate connection with a dev account
    onConnect('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
  };

  const truncate = (addr: string) =>
    `${addr.slice(0, 6)}…${addr.slice(-4)}`;

  if (connected && account) {
    return (
      <div className="wallet-connected">
        <div className="account-badge">
          <span className="account-dot" />
          <span className="account-address">{truncate(account)}</span>
        </div>
        <button className="btn btn-outline" onClick={onDisconnect}>
          Disconnect
        </button>
      </div>
    );
  }

  return (
    <button className="btn btn-primary" onClick={handleConnect}>
      <span>🔗</span> Connect Wallet
    </button>
  );
}
