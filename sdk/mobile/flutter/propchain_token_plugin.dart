// PropChain Flutter Plugin — Concrete Implementation
//
// Provides a ready-to-use implementation of [PropChainTokenHostApi] that
// communicates with a PropChain node via HTTP JSON-RPC using the web3dart
// package for offline transaction signing.
//
// Usage:
//   final plugin = PropChainTokenPlugin(rpcUrl: 'https://rpc.propchain.io');
//   final api = PropChainTokenFlutterApi(plugin);
//
//   final balance = await api.balanceOf('0xAbc...');
//   final txHash  = await api.transfer(from: '0xAbc...', to: '0xDef...', tokenId: 1, signerPrivateKey: '0x...');

import 'package:web3dart/web3dart.dart';
import 'package:http/http.dart' as http;
import 'propchain_token_api.dart';

/// Concrete implementation of [PropChainTokenHostApi].
///
/// Wraps [web3dart] to sign transactions offline and broadcast via RPC.
/// Mirrors the TypeScript [PropertyTokenClient] API surface.
class PropChainTokenPlugin implements PropChainTokenHostApi {
  final Web3Client _client;

  /// The deployed PropertyToken contract address.
  final EthereumAddress _contractAddress;

  /// Minimal ABI fragment covering balanceOf, transferFrom, approve,
  /// setApprovalForAll, getApproved, isApprovedForAll.
  static const List<Map<String, dynamic>> _abi = [
    {
      'name': 'balanceOf',
      'type': 'function',
      'stateMutability': 'view',
      'inputs': [
        {'name': 'owner', 'type': 'address'}
      ],
      'outputs': [
        {'name': '', 'type': 'uint256'}
      ],
    },
    {
      'name': 'transferFrom',
      'type': 'function',
      'stateMutability': 'nonpayable',
      'inputs': [
        {'name': 'from', 'type': 'address'},
        {'name': 'to', 'type': 'address'},
        {'name': 'tokenId', 'type': 'uint256'}
      ],
      'outputs': [],
    },
    {
      'name': 'approve',
      'type': 'function',
      'stateMutability': 'nonpayable',
      'inputs': [
        {'name': 'to', 'type': 'address'},
        {'name': 'tokenId', 'type': 'uint256'}
      ],
      'outputs': [],
    },
    {
      'name': 'setApprovalForAll',
      'type': 'function',
      'stateMutability': 'nonpayable',
      'inputs': [
        {'name': 'operator', 'type': 'address'},
        {'name': 'approved', 'type': 'bool'}
      ],
      'outputs': [],
    },
    {
      'name': 'getApproved',
      'type': 'function',
      'stateMutability': 'view',
      'inputs': [
        {'name': 'tokenId', 'type': 'uint256'}
      ],
      'outputs': [
        {'name': '', 'type': 'address'}
      ],
    },
    {
      'name': 'isApprovedForAll',
      'type': 'function',
      'stateMutability': 'view',
      'inputs': [
        {'name': 'owner', 'type': 'address'},
        {'name': 'operator', 'type': 'address'}
      ],
      'outputs': [
        {'name': '', 'type': 'bool'}
      ],
    },
  ];

  late final DeployedContract _contract;

  /// Creates a [PropChainTokenPlugin] connected to [rpcUrl] for the contract
  /// at [contractAddress].
  PropChainTokenPlugin({
    required String rpcUrl,
    required String contractAddress,
  })  : _client = Web3Client(rpcUrl, http.Client()),
        _contractAddress = EthereumAddress.fromHex(contractAddress) {
    _contract = DeployedContract(
      ContractAbi.fromJson(
        _abiJson,
        'PropertyToken',
      ),
      _contractAddress,
    );
  }

  // Internal JSON string of the ABI for ContractAbi.fromJson
  static final String _abiJson = (() {
    final buffer = StringBuffer('[');
    for (var i = 0; i < _abi.length; i++) {
      final entry = _abi[i];
      buffer.write('{');
      buffer.write('"name":"${entry['name']}",');
      buffer.write('"type":"${entry['type']}",');
      buffer.write('"stateMutability":"${entry['stateMutability']}",');

      // inputs
      final inputs = entry['inputs'] as List<Map<String, dynamic>>;
      buffer.write('"inputs":[');
      for (var j = 0; j < inputs.length; j++) {
        buffer.write('{"name":"${inputs[j]['name']}","type":"${inputs[j]['type']}"}');
        if (j < inputs.length - 1) buffer.write(',');
      }
      buffer.write('],');

      // outputs
      final outputs = entry['outputs'] as List<Map<String, dynamic>>;
      buffer.write('"outputs":[');
      for (var j = 0; j < outputs.length; j++) {
        buffer.write('{"name":"${outputs[j]['name']}","type":"${outputs[j]['type']}"}');
        if (j < outputs.length - 1) buffer.write(',');
      }
      buffer.write(']');

      buffer.write('}');
      if (i < _abi.length - 1) buffer.write(',');
    }
    buffer.write(']');
    return buffer.toString();
  })();

  // ---------------------------------------------------------------------------
  // Query helpers
  // ---------------------------------------------------------------------------

  ContractFunction _fn(String name) => _contract.function(name);

  Future<List<dynamic>> _call(String name, List<dynamic> params) async {
    return _client.call(
      contract: _contract,
      function: _fn(name),
      params: params,
    );
  }

  // ---------------------------------------------------------------------------
  // Write helpers
  // ---------------------------------------------------------------------------

  Future<String> _sendTx({
    required String signerPrivateKey,
    required String functionName,
    required List<dynamic> params,
  }) async {
    final credentials = EthPrivateKey.fromHex(signerPrivateKey);
    final chainId = await _client.getChainId();

    return _client.sendTransaction(
      credentials,
      Transaction.callContract(
        contract: _contract,
        function: _fn(functionName),
        parameters: params,
      ),
      chainId: chainId.toInt(),
    );
  }

  // ---------------------------------------------------------------------------
  // PropChainTokenHostApi implementation
  // ---------------------------------------------------------------------------

  @override
  Future<BalanceOfResult> balanceOf(BalanceOfRequest request) async {
    try {
      final result = await _call('balanceOf', [
        EthereumAddress.fromHex(request.owner),
      ]);
      final balance = (result[0] as BigInt).toInt();
      return BalanceOfResult(balance: balance);
    } catch (e) {
      throw PropChainTokenException('balanceOf failed: $e');
    }
  }

  @override
  Future<TransactionResult> transfer(TransferRequest request) async {
    try {
      final txHash = await _sendTx(
        signerPrivateKey: request.signerPrivateKey,
        functionName: 'transferFrom',
        params: [
          EthereumAddress.fromHex(request.from),
          EthereumAddress.fromHex(request.to),
          BigInt.from(request.tokenId),
        ],
      );
      return TransactionResult(success: true, txHash: txHash);
    } catch (e) {
      return TransactionResult(success: false, error: e.toString());
    }
  }

  @override
  Future<TransactionResult> approve(ApproveRequest request) async {
    try {
      final txHash = await _sendTx(
        signerPrivateKey: request.signerPrivateKey,
        functionName: 'approve',
        params: [
          EthereumAddress.fromHex(request.to),
          BigInt.from(request.tokenId),
        ],
      );
      return TransactionResult(success: true, txHash: txHash);
    } catch (e) {
      return TransactionResult(success: false, error: e.toString());
    }
  }

  @override
  Future<TransactionResult> setApprovalForAll(SetApprovalForAllRequest request) async {
    try {
      final txHash = await _sendTx(
        signerPrivateKey: request.signerPrivateKey,
        functionName: 'setApprovalForAll',
        params: [
          EthereumAddress.fromHex(request.operator),
          request.approved,
        ],
      );
      return TransactionResult(success: true, txHash: txHash);
    } catch (e) {
      return TransactionResult(success: false, error: e.toString());
    }
  }

  @override
  Future<GetApprovedResult> getApproved(int tokenId) async {
    try {
      final result = await _call('getApproved', [BigInt.from(tokenId)]);
      final addr = result[0] as EthereumAddress;
      // The zero address means no approval
      final isZero = addr == EthereumAddress.fromHex(
        '0x0000000000000000000000000000000000000000',
      );
      return GetApprovedResult(approvedAddress: isZero ? null : addr.hex);
    } catch (e) {
      throw PropChainTokenException('getApproved failed: $e');
    }
  }

  @override
  Future<IsApprovedForAllResult> isApprovedForAll(String owner, String operator) async {
    try {
      final result = await _call('isApprovedForAll', [
        EthereumAddress.fromHex(owner),
        EthereumAddress.fromHex(operator),
      ]);
      return IsApprovedForAllResult(isApproved: result[0] as bool);
    } catch (e) {
      throw PropChainTokenException('isApprovedForAll failed: $e');
    }
  }

  /// Releases underlying HTTP client resources. Call when plugin is no longer needed.
  void dispose() => _client.dispose();
}
