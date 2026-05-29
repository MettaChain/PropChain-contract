import React, { useState } from 'react';
import { ConnectWallet } from './components/ConnectWallet';
import { PropertyRegistry } from './components/PropertyRegistry';
import { EscrowManager } from './components/EscrowManager';
import { PropertyTokens } from './components/PropertyTokens';
import { LoadTestingDashboard } from './components/LoadTestingDashboard';

type TabId = 'properties' | 'escrow' | 'tokens' | 'loadtesting';

const TABS: { id: TabId; label: string; icon: string }[] = [
  { id: 'properties', label: 'Properties', icon: '🏠' },
  { id: 'escrow', label: 'Escrow', icon: '🔐' },
  { id: 'tokens', label: 'Tokens', icon: '🪙' },
  { id: 'loadtesting', label: 'Load Tests', icon: '📊' },
];

/**
 * Main application shell demonstrating PropChain SDK integration.
 */
export default function App() {
  const [activeTab, setActiveTab] = useState<TabId>('properties');
  const [connected, setConnected] = useState(false);
  const [account, setAccount] = useState<string | null>(null);

  const handleConnect = (address: string) => {
    setAccount(address);
    setConnected(true);
  };

  const handleDisconnect = () => {
    setAccount(null);
    setConnected(false);
  };

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="header-content">
          <div className="logo">
            <span className="logo-icon">⛓️</span>
            <h1>PropChain</h1>
            <span className="badge">SDK Demo</span>
          </div>
          <ConnectWallet
            connected={connected}
            account={account}
            onConnect={handleConnect}
            onDisconnect={handleDisconnect}
          />
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="tab-nav">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            className={`tab-btn ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            <span className="tab-icon">{tab.icon}</span>
            {tab.label}
          </button>
        ))}
      </nav>

      {/* Main Content */}
      <main className="main-content">
        {activeTab === 'loadtesting' ? (
          <LoadTestingDashboard />
        ) : !connected ? (
          <div className="connect-prompt">
            <div className="prompt-card">
              <span className="prompt-icon">🔗</span>
              <h2>Connect Your Wallet</h2>
              <p>
                Connect your Polkadot.js wallet to interact with PropChain smart contracts.
                This example app demonstrates the full SDK capabilities.
              </p>
              <div className="feature-grid">
                <div className="feature-item">
                  <span>🏠</span>
                  <strong>Property Registry</strong>
                  <p>Register, transfer, and manage properties</p>
                </div>
                <div className="feature-item">
                  <span>🔐</span>
                  <strong>Escrow</strong>
                  <p>Secure property transactions with escrow</p>
                </div>
                <div className="feature-item">
                  <span>🪙</span>
                  <strong>Property Tokens</strong>
                  <p>NFTs, fractional ownership, governance</p>
                </div>
                <div className="feature-item">
                  <span>⛓️</span>
                  <strong>Cross-Chain</strong>
                  <p>Bridge property tokens across chains</p>
                </div>
                <div className="feature-item">
                  <span>📊</span>
                  <strong>Load Testing</strong>
                  <p>View CI/CD performance metrics</p>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <>
            {activeTab === 'properties' && <PropertyRegistry account={account!} />}
            {activeTab === 'escrow' && <EscrowManager account={account!} />}
            {activeTab === 'tokens' && <PropertyTokens account={account!} />}
          </>
        )}
      </main>

      {/* Footer */}
      <footer className="footer">
        <p>
          PropChain SDK v0.1.0 — Built with{' '}
          <a href="https://polkadot.js.org/" target="_blank" rel="noopener noreferrer">
            Polkadot.js
          </a>{' '}
          on Substrate
        </p>
      </footer>
    </div>
  );
}
