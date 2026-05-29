import React, { useState } from 'react';

interface PropertyTokensProps {
  account: string;
}

/**
 * Property Tokens component demonstrating:
 * - Minting property NFTs
 * - Fractional ownership (share issuance/redemption)
 * - Governance proposals and voting
 * - Secondary marketplace (asks, purchases)
 *
 * @example SDK usage:
 * ```typescript
 * // Mint a property token
 * const { tokenId } = await client.propertyToken
 *   .registerPropertyWithToken(signer, metadata);
 *
 * // Issue fractional shares
 * await client.propertyToken.issueShares(signer, tokenId, to, amount);
 *
 * // Create governance proposal
 * const { proposalId } = await client.propertyToken
 *   .createProposal(signer, tokenId, descHash, quorum);
 * ```
 */
export function PropertyTokens({ account }: PropertyTokensProps) {
  const [activeSection, setActiveSection] = useState<'nft' | 'fractional' | 'governance' | 'market'>('nft');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const sections = [
    { id: 'nft' as const, label: 'NFT Tokens', icon: '🎨' },
    { id: 'fractional' as const, label: 'Fractional', icon: '📊' },
    { id: 'governance' as const, label: 'Governance', icon: '🗳️' },
    { id: 'market' as const, label: 'Marketplace', icon: '💹' },
  ];

  const simulateAction = async (actionName: string) => {
    setLoading(true);
    setMessage(null);
    try {
      await new Promise((resolve) => setTimeout(resolve, 1200));
      setMessage({ type: 'success', text: `${actionName} completed successfully!` });
    } catch (err) {
      setMessage({ type: 'error', text: `${actionName} failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="panel">
      <div className="panel-header">
        <h2>🪙 Property Tokens</h2>
        <p className="subtitle">NFTs, fractional ownership, governance, and marketplace</p>
      </div>

      {/* Sub-navigation */}
      <div className="sub-nav">
        {sections.map((section) => (
          <button
            key={section.id}
            className={`sub-nav-btn ${activeSection === section.id ? 'active' : ''}`}
            onClick={() => setActiveSection(section.id)}
          >
            <span>{section.icon}</span> {section.label}
          </button>
        ))}
      </div>

      {/* NFT Section */}
      {activeSection === 'nft' && (
        <div className="two-column">
          <div className="card">
            <h3>🎨 Mint Property Token</h3>
            <p className="card-desc">
              Mint an ERC-721 compatible NFT representing a property.
              Each token includes property metadata, legal documents,
              and compliance verification.
            </p>
            <button
              className="btn btn-primary btn-full"
              onClick={() => simulateAction('Token minting')}
              disabled={loading}
            >
              {loading ? '⏳ Minting...' : '🎨 Mint Token'}
            </button>
          </div>
          <div className="card">
            <h3>SDK Code</h3>
            <pre className="code-block">
{`// Mint property token
const { tokenId } = await client
  .propertyToken
  .registerPropertyWithToken(signer, {
    location: '123 Main St',
    size: 2500,
    legalDescription: 'Lot 1',
    valuation: BigInt(500000_00000000),
    documentsUrl: 'ipfs://Qm...',
  });

// Attach legal document
await client.propertyToken
  .attachLegalDocument(
    signer, tokenId,
    documentHash, 'deed'
  );

// Get ownership history
const history = await client
  .propertyToken
  .getOwnershipHistory(tokenId);`}
            </pre>
          </div>
        </div>
      )}

      {/* Fractional Section */}
      {activeSection === 'fractional' && (
        <div className="two-column">
          <div className="card">
            <h3>📊 Fractional Ownership</h3>
            <p className="card-desc">
              Issue fractional shares of property tokens. Share holders
              receive dividends proportional to their stake and can
              participate in governance.
            </p>
            <div className="btn-row">
              <button
                className="btn btn-primary"
                onClick={() => simulateAction('Share issuance')}
                disabled={loading}
              >
                📤 Issue Shares
              </button>
              <button
                className="btn btn-secondary"
                onClick={() => simulateAction('Dividend deposit')}
                disabled={loading}
              >
                💰 Deposit Dividends
              </button>
            </div>
          </div>
          <div className="card">
            <h3>SDK Code</h3>
            <pre className="code-block">
{`// Issue fractional shares
await client.propertyToken
  .issueShares(signer, tokenId,
    recipientAddr, BigInt(1000));

// Deposit dividends
await client.propertyToken
  .depositDividends(signer, tokenId,
    BigInt(10000));

// Distribute recurring rental income
await client.propertyToken
  .distributeRentalIncome(signer, tokenId,
    BigInt(5000));

// Check share balance
const balance = await client
  .propertyToken
  .getShareBalance(tokenId, account);

// Withdraw dividends
await client.propertyToken
  .withdrawDividends(signer, tokenId);`}
            </pre>
          </div>
        </div>
      )}

      {/* Governance Section */}
      {activeSection === 'governance' && (
        <div className="two-column">
          <div className="card">
            <h3>🗳️ On-chain Governance</h3>
            <p className="card-desc">
              Share holders can create and vote on proposals for
              property management decisions. Voting weight is
              proportional to share ownership.
            </p>
            <div className="btn-row">
              <button
                className="btn btn-primary"
                onClick={() => simulateAction('Proposal creation')}
                disabled={loading}
              >
                📋 Create Proposal
              </button>
              <button
                className="btn btn-success"
                onClick={() => simulateAction('Vote cast')}
                disabled={loading}
              >
                ✅ Vote For
              </button>
              <button
                className="btn btn-danger"
                onClick={() => simulateAction('Vote cast')}
                disabled={loading}
              >
                ❌ Vote Against
              </button>
            </div>
          </div>
          <div className="card">
            <h3>SDK Code</h3>
            <pre className="code-block">
{`// Create governance proposal
const { proposalId } = await client
  .propertyToken.createProposal(
    signer, tokenId,
    descriptionHash,
    BigInt(500) // quorum
  );

// Vote on proposal
await client.propertyToken.vote(
  signer, tokenId,
  proposalId, true // support
);

// Execute passed proposal
await client.propertyToken
  .executeProposal(
    signer, tokenId, proposalId
  );`}
            </pre>
          </div>
        </div>
      )}

      {/* Marketplace Section */}
      {activeSection === 'market' && (
        <div className="two-column">
          <div className="card">
            <h3>💹 Secondary Marketplace</h3>
            <p className="card-desc">
              Trade fractional shares on the built-in secondary market.
              Place sell orders and buy shares from other holders.
            </p>
            <div className="btn-row">
              <button
                className="btn btn-primary"
                onClick={() => simulateAction('Ask placement')}
                disabled={loading}
              >
                📈 Place Ask
              </button>
              <button
                className="btn btn-success"
                onClick={() => simulateAction('Share purchase')}
                disabled={loading}
              >
                🛒 Buy Shares
              </button>
            </div>
          </div>
          <div className="card">
            <h3>SDK Code</h3>
            <pre className="code-block">
{`// Place a sell ask
await client.propertyToken.placeAsk(
  signer, tokenId,
  BigInt(100), // price per share
  BigInt(50)   // amount
);

// Buy shares from a seller
await client.propertyToken.buyShares(
  signer, tokenId,
  sellerAddress,
  BigInt(25), // number of shares
  BigInt(2500), // payment attached
);

// Check last trade price
const price = await client
  .propertyToken
  .getLastTradePrice(tokenId);`}
            </pre>
          </div>
        </div>
      )}

      {message && (
        <div className={`message message-${message.type}`}>
          {message.type === 'success' ? '✅' : '❌'} {message.text}
        </div>
      )}
    </div>
  );
}
