// PropChain Flutter Plugin — Pigeon-style API definitions
//
// This file defines the message classes and abstract API interfaces for the
// PropChain property-token Flutter plugin. It follows the Pigeon pattern:
// message classes carry typed data, PropChainTokenHostApi is the platform
// interface (implemented natively or in Dart), and PropChainTokenFlutterApi
// is the Flutter-side caller.
//
// Covered operations:
//   - balanceOf    — query token balance for an owner address
//   - transfer     — transfer a property token from one account to another
//   - approve      — approve an account to transfer a specific token
//   - setApprovalForAll — grant/revoke operator for all tokens
//   - getApproved  — query the approved address for a token
//   - isApprovedForAll — query operator approval status

// ---------------------------------------------------------------------------
// Transfer
// ---------------------------------------------------------------------------

/// Request payload for a property token transfer.
class TransferRequest {
  /// The sender address (account initiating the transfer).
  final String from;

  /// The recipient address.
  final String to;

  /// The property token ID to transfer.
  final int tokenId;

  /// Hex-encoded private key used to sign the transaction offline.
  /// Store securely using OS keychain / Flutter Secure Storage.
  final String signerPrivateKey;

  const TransferRequest({
    required this.from,
    required this.to,
    required this.tokenId,
    required this.signerPrivateKey,
  });
}

/// Result of a transfer or approval transaction.
class TransactionResult {
  /// The on-chain transaction hash, or null on failure.
  final String? txHash;

  /// Whether the transaction was submitted successfully.
  final bool success;

  /// Human-readable error message if [success] is false.
  final String? error;

  const TransactionResult({
    required this.success,
    this.txHash,
    this.error,
  });
}

// ---------------------------------------------------------------------------
// Approval
// ---------------------------------------------------------------------------

/// Request payload to approve a single address for a specific token.
class ApproveRequest {
  /// The address being granted approval.
  final String to;

  /// The property token ID being approved.
  final int tokenId;

  /// Hex-encoded private key of the current owner signing the approval.
  final String signerPrivateKey;

  const ApproveRequest({
    required this.to,
    required this.tokenId,
    required this.signerPrivateKey,
  });
}

/// Request payload to set/revoke operator approval for all tokens.
class SetApprovalForAllRequest {
  /// The operator address.
  final String operator;

  /// Whether to grant (true) or revoke (false) approval.
  final bool approved;

  /// Hex-encoded private key of the owner signing the transaction.
  final String signerPrivateKey;

  const SetApprovalForAllRequest({
    required this.operator,
    required this.approved,
    required this.signerPrivateKey,
  });
}

// ---------------------------------------------------------------------------
// Balance / Query
// ---------------------------------------------------------------------------

/// Request payload to query the token balance of an owner.
class BalanceOfRequest {
  /// The owner address to query.
  final String owner;

  const BalanceOfRequest({required this.owner});
}

/// Result of a balanceOf query.
class BalanceOfResult {
  /// Number of tokens owned by the queried address.
  final int balance;

  const BalanceOfResult({required this.balance});
}

/// Result of a getApproved query.
class GetApprovedResult {
  /// The approved address for the token, or null if none.
  final String? approvedAddress;

  const GetApprovedResult({this.approvedAddress});
}

/// Result of an isApprovedForAll query.
class IsApprovedForAllResult {
  /// Whether the operator is approved for all tokens of the owner.
  final bool isApproved;

  const IsApprovedForAllResult({required this.isApproved});
}

// ---------------------------------------------------------------------------
// Abstract Host API (platform / Dart implementation side)
// ---------------------------------------------------------------------------

/// The platform-side interface for the PropChain token plugin.
///
/// In a true Pigeon setup, this would be implemented natively (Swift/Kotlin).
/// In this pure-Dart SDK, implementors provide the RPC/signing logic.
abstract class PropChainTokenHostApi {
  /// Returns the number of property tokens owned by [request.owner].
  Future<BalanceOfResult> balanceOf(BalanceOfRequest request);

  /// Transfers [request.tokenId] from [request.from] to [request.to].
  Future<TransactionResult> transfer(TransferRequest request);

  /// Approves [request.to] to transfer [request.tokenId].
  Future<TransactionResult> approve(ApproveRequest request);

  /// Grants or revokes [request.operator] approval for all caller tokens.
  Future<TransactionResult> setApprovalForAll(SetApprovalForAllRequest request);

  /// Returns the approved address for [tokenId], or null if none.
  Future<GetApprovedResult> getApproved(int tokenId);

  /// Returns whether [operator] is approved for all tokens of [owner].
  Future<IsApprovedForAllResult> isApprovedForAll(String owner, String operator);
}

// ---------------------------------------------------------------------------
// Flutter API (caller-side convenience wrapper)
// ---------------------------------------------------------------------------

/// Flutter-side API for calling PropChain token operations.
///
/// Wraps [PropChainTokenHostApi] and exposes ergonomic async methods.
/// Errors propagate as [PropChainTokenException].
class PropChainTokenFlutterApi {
  final PropChainTokenHostApi _host;

  const PropChainTokenFlutterApi(this._host);

  /// Query the number of tokens owned by [owner].
  Future<int> balanceOf(String owner) async {
    final result = await _host.balanceOf(BalanceOfRequest(owner: owner));
    return result.balance;
  }

  /// Transfer [tokenId] from [from] to [to], signed with [signerPrivateKey].
  ///
  /// Returns the on-chain transaction hash on success.
  /// Throws [PropChainTokenException] on failure.
  Future<String> transfer({
    required String from,
    required String to,
    required int tokenId,
    required String signerPrivateKey,
  }) async {
    final result = await _host.transfer(TransferRequest(
      from: from,
      to: to,
      tokenId: tokenId,
      signerPrivateKey: signerPrivateKey,
    ));
    if (!result.success || result.txHash == null) {
      throw PropChainTokenException(result.error ?? 'Transfer failed');
    }
    return result.txHash!;
  }

  /// Approve [to] to transfer [tokenId], signed with [signerPrivateKey].
  ///
  /// Returns the on-chain transaction hash on success.
  Future<String> approve({
    required String to,
    required int tokenId,
    required String signerPrivateKey,
  }) async {
    final result = await _host.approve(ApproveRequest(
      to: to,
      tokenId: tokenId,
      signerPrivateKey: signerPrivateKey,
    ));
    if (!result.success || result.txHash == null) {
      throw PropChainTokenException(result.error ?? 'Approval failed');
    }
    return result.txHash!;
  }

  /// Grant or revoke [operator] for all caller tokens.
  ///
  /// Returns the on-chain transaction hash on success.
  Future<String> setApprovalForAll({
    required String operator,
    required bool approved,
    required String signerPrivateKey,
  }) async {
    final result = await _host.setApprovalForAll(SetApprovalForAllRequest(
      operator: operator,
      approved: approved,
      signerPrivateKey: signerPrivateKey,
    ));
    if (!result.success || result.txHash == null) {
      throw PropChainTokenException(result.error ?? 'SetApprovalForAll failed');
    }
    return result.txHash!;
  }

  /// Returns the approved address for [tokenId], or null if none.
  Future<String?> getApproved(int tokenId) async {
    final result = await _host.getApproved(tokenId);
    return result.approvedAddress;
  }

  /// Returns whether [operator] is approved for all tokens of [owner].
  Future<bool> isApprovedForAll(String owner, String operator) async {
    final result = await _host.isApprovedForAll(owner, operator);
    return result.isApproved;
  }
}

// ---------------------------------------------------------------------------
// Exception
// ---------------------------------------------------------------------------

/// Thrown by [PropChainTokenFlutterApi] when a token operation fails.
class PropChainTokenException implements Exception {
  final String message;
  const PropChainTokenException(this.message);

  @override
  String toString() => 'PropChainTokenException: $message';
}
