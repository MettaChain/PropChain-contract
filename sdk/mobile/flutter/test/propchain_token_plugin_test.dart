// PropChain Flutter Plugin — Unit Tests
//
// Tests the [PropChainTokenFlutterApi] caller-side logic using a mock
// [PropChainTokenHostApi]. Does not require a live RPC node.

import 'package:test/test.dart';
import '../propchain_token_api.dart';

// ---------------------------------------------------------------------------
// Mock Host API
// ---------------------------------------------------------------------------

class MockPropChainTokenHostApi implements PropChainTokenHostApi {
  // Configurable responses
  int mockBalance = 3;
  String mockTxHash = '0xdeadbeef1234567890abcdef';
  String? mockApprovedAddress = '0xApprovedAddress';
  bool mockIsApprovedForAll = true;

  // Error simulation flags
  bool shouldFailTransfer = false;
  bool shouldFailApprove = false;
  bool shouldFailSetApprovalForAll = false;

  // Captured call params for assertion
  BalanceOfRequest? lastBalanceOfRequest;
  TransferRequest? lastTransferRequest;
  ApproveRequest? lastApproveRequest;
  SetApprovalForAllRequest? lastSetApprovalForAllRequest;
  int? lastGetApprovedTokenId;
  String? lastIsApprovedOwner;
  String? lastIsApprovedOperator;

  @override
  Future<BalanceOfResult> balanceOf(BalanceOfRequest request) async {
    lastBalanceOfRequest = request;
    return BalanceOfResult(balance: mockBalance);
  }

  @override
  Future<TransactionResult> transfer(TransferRequest request) async {
    lastTransferRequest = request;
    if (shouldFailTransfer) {
      return const TransactionResult(success: false, error: 'Insufficient balance');
    }
    return TransactionResult(success: true, txHash: mockTxHash);
  }

  @override
  Future<TransactionResult> approve(ApproveRequest request) async {
    lastApproveRequest = request;
    if (shouldFailApprove) {
      return const TransactionResult(success: false, error: 'Approval rejected');
    }
    return TransactionResult(success: true, txHash: mockTxHash);
  }

  @override
  Future<TransactionResult> setApprovalForAll(SetApprovalForAllRequest request) async {
    lastSetApprovalForAllRequest = request;
    if (shouldFailSetApprovalForAll) {
      return const TransactionResult(success: false, error: 'SetApprovalForAll failed');
    }
    return TransactionResult(success: true, txHash: mockTxHash);
  }

  @override
  Future<GetApprovedResult> getApproved(int tokenId) async {
    lastGetApprovedTokenId = tokenId;
    return GetApprovedResult(approvedAddress: mockApprovedAddress);
  }

  @override
  Future<IsApprovedForAllResult> isApprovedForAll(String owner, String operator) async {
    lastIsApprovedOwner = owner;
    lastIsApprovedOperator = operator;
    return IsApprovedForAllResult(isApproved: mockIsApprovedForAll);
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

void main() {
  late MockPropChainTokenHostApi mockHost;
  late PropChainTokenFlutterApi api;

  const ownerAddress = '0xOwnerAddress123';
  const toAddress = '0xToAddress456';
  const operatorAddress = '0xOperatorAddress789';
  const privateKey = '0xprivatekey';
  const tokenId = 42;

  setUp(() {
    mockHost = MockPropChainTokenHostApi();
    api = PropChainTokenFlutterApi(mockHost);
  });

  // -------------------------------------------------------------------------
  // balanceOf
  // -------------------------------------------------------------------------
  group('balanceOf', () {
    test('returns balance from host api', () async {
      mockHost.mockBalance = 5;
      final balance = await api.balanceOf(ownerAddress);
      expect(balance, equals(5));
      expect(mockHost.lastBalanceOfRequest?.owner, equals(ownerAddress));
    });

    test('returns 0 balance correctly', () async {
      mockHost.mockBalance = 0;
      final balance = await api.balanceOf(ownerAddress);
      expect(balance, equals(0));
    });
  });

  // -------------------------------------------------------------------------
  // transfer
  // -------------------------------------------------------------------------
  group('transfer', () {
    test('returns txHash on successful transfer', () async {
      final txHash = await api.transfer(
        from: ownerAddress,
        to: toAddress,
        tokenId: tokenId,
        signerPrivateKey: privateKey,
      );
      expect(txHash, equals(mockHost.mockTxHash));
      expect(mockHost.lastTransferRequest?.from, equals(ownerAddress));
      expect(mockHost.lastTransferRequest?.to, equals(toAddress));
      expect(mockHost.lastTransferRequest?.tokenId, equals(tokenId));
    });

    test('throws PropChainTokenException on failure', () async {
      mockHost.shouldFailTransfer = true;
      expect(
        () => api.transfer(
          from: ownerAddress,
          to: toAddress,
          tokenId: tokenId,
          signerPrivateKey: privateKey,
        ),
        throwsA(isA<PropChainTokenException>().having(
          (e) => e.message,
          'message',
          contains('Insufficient balance'),
        )),
      );
    });
  });

  // -------------------------------------------------------------------------
  // approve
  // -------------------------------------------------------------------------
  group('approve', () {
    test('returns txHash on successful approval', () async {
      final txHash = await api.approve(
        to: toAddress,
        tokenId: tokenId,
        signerPrivateKey: privateKey,
      );
      expect(txHash, equals(mockHost.mockTxHash));
      expect(mockHost.lastApproveRequest?.to, equals(toAddress));
      expect(mockHost.lastApproveRequest?.tokenId, equals(tokenId));
    });

    test('throws PropChainTokenException on failure', () async {
      mockHost.shouldFailApprove = true;
      expect(
        () => api.approve(
          to: toAddress,
          tokenId: tokenId,
          signerPrivateKey: privateKey,
        ),
        throwsA(isA<PropChainTokenException>().having(
          (e) => e.message,
          'message',
          contains('Approval rejected'),
        )),
      );
    });
  });

  // -------------------------------------------------------------------------
  // setApprovalForAll
  // -------------------------------------------------------------------------
  group('setApprovalForAll', () {
    test('returns txHash when granting operator approval', () async {
      final txHash = await api.setApprovalForAll(
        operator: operatorAddress,
        approved: true,
        signerPrivateKey: privateKey,
      );
      expect(txHash, equals(mockHost.mockTxHash));
      expect(mockHost.lastSetApprovalForAllRequest?.operator, equals(operatorAddress));
      expect(mockHost.lastSetApprovalForAllRequest?.approved, isTrue);
    });

    test('returns txHash when revoking operator approval', () async {
      final txHash = await api.setApprovalForAll(
        operator: operatorAddress,
        approved: false,
        signerPrivateKey: privateKey,
      );
      expect(txHash, equals(mockHost.mockTxHash));
      expect(mockHost.lastSetApprovalForAllRequest?.approved, isFalse);
    });

    test('throws PropChainTokenException on failure', () async {
      mockHost.shouldFailSetApprovalForAll = true;
      expect(
        () => api.setApprovalForAll(
          operator: operatorAddress,
          approved: true,
          signerPrivateKey: privateKey,
        ),
        throwsA(isA<PropChainTokenException>()),
      );
    });
  });

  // -------------------------------------------------------------------------
  // getApproved
  // -------------------------------------------------------------------------
  group('getApproved', () {
    test('returns approved address for a token', () async {
      final address = await api.getApproved(tokenId);
      expect(address, equals(mockHost.mockApprovedAddress));
      expect(mockHost.lastGetApprovedTokenId, equals(tokenId));
    });

    test('returns null when no address is approved', () async {
      mockHost.mockApprovedAddress = null;
      final address = await api.getApproved(tokenId);
      expect(address, isNull);
    });
  });

  // -------------------------------------------------------------------------
  // isApprovedForAll
  // -------------------------------------------------------------------------
  group('isApprovedForAll', () {
    test('returns true when operator is approved', () async {
      mockHost.mockIsApprovedForAll = true;
      final result = await api.isApprovedForAll(ownerAddress, operatorAddress);
      expect(result, isTrue);
      expect(mockHost.lastIsApprovedOwner, equals(ownerAddress));
      expect(mockHost.lastIsApprovedOperator, equals(operatorAddress));
    });

    test('returns false when operator is not approved', () async {
      mockHost.mockIsApprovedForAll = false;
      final result = await api.isApprovedForAll(ownerAddress, operatorAddress);
      expect(result, isFalse);
    });
  });

  // -------------------------------------------------------------------------
  // PropChainTokenException
  // -------------------------------------------------------------------------
  group('PropChainTokenException', () {
    test('toString includes the error message', () {
      const ex = PropChainTokenException('Something went wrong');
      expect(ex.toString(), contains('Something went wrong'));
    });
  });
}
