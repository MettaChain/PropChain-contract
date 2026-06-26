# PropChain Mobile SDK Integration Guide

This documentation provides guidance for integrating the PropChain Mobile SDK into React Native and Flutter applications.

## Features
- Mobile-optimized contract interface
- Offline transaction signing
- QR code scanning for property info
- Push notification system
- Biometric authentication
- Mobile-specific error handling

## Directory Structure
- `sdk/mobile/common/` — Shared interfaces and logic
- `sdk/mobile/react-native/` — React Native SDK implementation
- `sdk/mobile/flutter/` — Flutter SDK implementation

## Integration Steps

### React Native
1. Install dependencies: `ethers`, `expo-barcode-scanner`, `expo-local-authentication`, `@react-native-firebase/messaging`, etc.
2. Import SDK modules from `sdk/mobile/react-native/`.
3. Use the provided interfaces and utilities to interact with PropChain contracts.

### Flutter
1. Add dependencies: `web3dart`, `qr_code_scanner`, `local_auth`, `firebase_messaging`, etc.
2. Import SDK modules from `sdk/mobile/flutter/`.
3. Use the provided interfaces and utilities to interact with PropChain contracts.

## Sample Apps
See `sdk/mobile/react-native/sample-app/` and `sdk/mobile/flutter/sample_app/` for starter templates.

## Error Handling & Recovery
- Use the provided error handling hooks to catch and recover from mobile-specific issues.

## Security
- Always store private keys securely (use OS keychain/secure storage).
- Use biometric authentication for sensitive actions.

## Flutter Token Plugin

The `sdk/mobile/flutter/propchain_token_plugin.dart` exposes a Pigeon-style API for
`transfer`, `approval`, and `balanceOf` operations on PropChain property tokens.

### Installation

Add to your Flutter project's `pubspec.yaml`:

```yaml
dependencies:
  web3dart: ^2.7.3
  http: ^1.2.0
```

### Usage

```dart
import 'package:your_app/propchain_token_plugin.dart';
import 'package:your_app/propchain_token_api.dart';

// 1. Create the plugin (connects to your RPC endpoint)
final plugin = PropChainTokenPlugin(
  rpcUrl: 'https://rpc.propchain.io',
  contractAddress: '0xYourPropertyTokenContract',
);

// 2. Wrap with the Flutter API
final tokenApi = PropChainTokenFlutterApi(plugin);

// Query token balance
final balance = await tokenApi.balanceOf('0xOwnerAddress');
print('Balance: $balance tokens');

// Transfer a token (tokenId = 42)
final txHash = await tokenApi.transfer(
  from: '0xOwnerAddress',
  to: '0xRecipientAddress',
  tokenId: 42,
  signerPrivateKey: securelyFetchedPrivateKey, // use Flutter Secure Storage
);
print('Transfer tx: $txHash');

// Approve another address to transfer token #42
final approveTx = await tokenApi.approve(
  to: '0xSpenderAddress',
  tokenId: 42,
  signerPrivateKey: securelyFetchedPrivateKey,
);
print('Approval tx: $approveTx');

// Grant operator approval for all tokens
final setAllTx = await tokenApi.setApprovalForAll(
  operator: '0xOperatorAddress',
  approved: true,
  signerPrivateKey: securelyFetchedPrivateKey,
);
print('SetApprovalForAll tx: $setAllTx');

// Query approved address for a token
final approvedAddress = await tokenApi.getApproved(42);
print('Approved address: $approvedAddress');

// Query operator approval status
final isApproved = await tokenApi.isApprovedForAll('0xOwner', '0xOperator');
print('Is operator approved: $isApproved');

// Clean up when done
plugin.dispose();
```

### Error Handling

All write operations throw `PropChainTokenException` on failure:

```dart
try {
  await tokenApi.transfer(
    from: '0xOwner',
    to: '0xRecipient',
    tokenId: 1,
    signerPrivateKey: privateKey,
  );
} on PropChainTokenException catch (e) {
  print('Transfer failed: ${e.message}');
}
```

> **Security:** Never hardcode private keys. Use [`flutter_secure_storage`](https://pub.dev/packages/flutter_secure_storage) or the OS keychain to store and retrieve signing keys.

## React Native — Property Minting

The `sdk/mobile/react-native/mint_property.ts` file exposes an opinionated API for the full `mint_property` flow covering KYC, offline signing, and progress streaming.

### Usage

```typescript
import { mintProperty, MintProgressStatus } from './mint_property';

const result = await mintProperty(
  'https://rpc.propchain.io',
  '0xPropertyTokenContract',
  privateKey,           // from react-native-keychain
  {
    location: '123 Marina Drive, Lagos',
    sizeSqm: 450,
    legalDescription: 'Lot 7, Block 3, Marina Estate',
    valuation: BigInt('5000000000000000000'),
    documentsUri: 'ipfs://QmXyz...',
  },
  {
    kycProviderUrl: 'https://kyc.propchain.io',
    onProgress: ({ status, message }) => {
      if (status === MintProgressStatus.KycChecking) showSpinner('Verifying KYC…');
      if (status === MintProgressStatus.Finalized) showSuccess(`Token #${result.tokenId} minted!`);
    },
  },
);
```

## Support
For questions or issues, see the main project README or contact the maintainers.
