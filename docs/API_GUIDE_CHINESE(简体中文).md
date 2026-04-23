# PropChain API 文档指南

## 概述

本指南为开发人员提供了用于集成 PropChain 智能合约的完整、详细的 API 文档。它遵循 [API_DOCUMENTATION_STANDARDS.md](./API_DOCUMENTATION_STANDARDS.md) 中定义的标准，并包含来自 [API_ERROR_CODES.md](./API_ERROR_CODES.md) 的全面错误文档。

---

## 快速入门

### 1. 找到您需要的东西

**按用例分类**:
- **注册房产 (Register Property)**: 请参阅 [`register_property`](#register_property)
- **转移所有权 (Transfer Ownership)**: 请参阅 [`transfer_property`](#transfer_property)
- **检查合规性 (Check Compliance)**: 请参阅 [`check_account_compliance`](#check_account_compliance)
- **创建托管 (Create Escrow)**: 请参阅 [Escrow Contract](#escrow-contract)
- **获取估价 (Get Valuation)**: 请参阅 [Oracle Contract](#oracle-contract)

**按角色分类**:
- **前端开发者 (Frontend Developer)**: 从示例和基本操作开始
- **后端开发者 (Backend Developer)**: 重点关注事件和状态查询
- **智能合约开发者 (Smart Contract Dev)**: 查看集成模式和跨合约调用
- **审计员 (Auditor)**: 研究错误处理和安全要求

---

## 核心 API 参考

### 产权登记合同

物业管理和所有权跟踪的主要合同

#### 构造函数

##### `new()`

创建并初始化一个新的 PropertyRegistry 合约实例。

**文档**: 请参阅源代码中的详细 rustdoc。
**例子**:
```rust
// 自动部署 - 无需手动调用
let contract = PropertyRegistry::new();
assert_eq!(contract.version(), 1);
```

---

#### 只读函数（视图方法）

这些函数不会修改状态，可以自由调用。

##### `version() -> u32`

返回合约版本号（当前为 1）。

**参数**: 没有任何  
**退货**: `u32` - 版本号（当前为 1）
**天然气成本**: ~500 气体  
**例子**:
```rust
let version = contract.version();
if version >= 2 {
    // 使用新功能
}
```

---

##### `admin() -> AccountId`

返回管理员账户地址。

**参数**: 没有任何  
**退货**: `AccountId` - 管理员的 Substrate 帐户
**天然气成本**: ~500 气体  
**例子**:
```rust
let admin = contract.admin();
println!("Contract admin: {:?}", admin);
```

---

##### `health_check() -> HealthStatus`

提供全面的健康状态以供监控（包括属性总数、活跃托管和预言机配置等）。

**参数**: 没有任何  
**退货**: [`HealthStatus`](crate::HealthStatus) 结构为：
- `is_healthy: bool` - 整体健康标志
- `is_paused: bool` - 暂停状态
- `contract_version: u32` - 版本号
- `property_count: u64` - 总资产
- `escrow_count: u64` - 活跃的托管账户
- `has_oracle: bool` - Oracle已配置
- `has_compliance_registry: bool` - 配置合规性
- `has_fee_manager: bool` - 已配置费用管理器
- `block_number: u32` - 当前区块
- `timestamp: u64` - 当前时间戳

**天然气成本**: ~2,000 气体  
**例子**:
```rust
let health = contract.health_check();
if !health.is_healthy {
    alert_admins("Contract issues detected!");
}
println!("Properties: {}", health.property_count);
```

---

##### `ping() -> bool`

简单的活体检测。

**参数**: 没有任何  
**退货**: `bool` - 总是回来 `true` 如果合同有效
**天然气成本**: ~500 气体  
**使用案例**: 确认合同已部署并正常运行

---

##### `dependencies_healthy() -> bool`

检查所有关键依赖项是否已配置。

**参数**: 没有任何  
**退货**: `bool` - `true` 如果 Oracle、合规性和费用管理器都已配置
**天然气成本**: ~1,000 气体  
**例子**:
```rust
if contract.dependencies_healthy() {
    println!("All systems operational");
} else {
    println!("Some dependencies not configured");
}
```

---

##### `oracle() -> Option<AccountId>`

返回预言机合约地址。

**参数**: 没有任何  
**退货**: `Option<AccountId>` - Oracle 地址（如果已配置）
**天然气成本**: ~500 气体  

---

##### `get_fee_manager() -> Option<AccountId>`

返回费用管理合同地址。

**参数**: 没有任何  
**退货**: `Option<AccountId>` - 如果已配置，费用管理地址
**天然气成本**: ~500 气体  

---

##### `get_compliance_registry() -> Option<AccountId>`

返回合规注册合同地址。

**参数**: 没有任何  
**退货**: `Option<AccountId>` - 如果已配置，则提供合规性注册表地址。
**天然气成本**: ~500 气体 

---

##### `check_account_compliance(account: AccountId) -> Result<bool, Error>`

检查账户是否符合合规性要求。

**参数**:
- `account` (`AccountId`) - 要检查的帐户

**退货**:
- `Ok(bool)` - `true` 如果符合, `false` 否则
- `Err(Error)` - 如果合规性检查技术上失败

**错误**:
- [`Error::ComplianceCheckFailed`](./API_ERROR_CODES.md#error-compliancecheckfailed) - 注册表调用失败
- [`Error::OracleError`](./API_ERROR_CODES.md#error-oracleerror) - 跨合约期权交易失败

**天然气成本**: ~5,000 天然气（包括交叉合约期权）
**例子**:
```rust
match contract.check_account_compliance(buyer_account) {
    Ok(true) => println!("Account is compliant"),
    Ok(false) => println!("Account NOT compliant - needs KYC"),
    Err(e) => eprintln!("Compliance check error: {:?}", e),
}
```

---

##### `get_dynamic_fee(operation: FeeOperation) -> u128`

返回特定操作的动态费用。

**参数**:
- `operation` (`FeeOperation`) - 操作类型

**退货**:
- `u128` - 手续费金额（以最小货币单位计，单位为分）

**天然气成本**: ~3,000 气体  
**例子**:
```rust
let fee = contract.get_dynamic_fee(FeeOperation::PropertyTransfer);
println!("Transfer fee: {} cents", fee);
```

---

#### 状态变更函数（事务）

这些功能会修改合约状态，并且需要消耗 gas。

##### `change_admin(new_admin: AccountId) -> Result<(), Error>`

将管理员权限转移到新帐户。

**参数**:
- `new_admin` (`AccountId`) - 获得管理员权限的帐户
  - **格式**: 32 字节的 Substrate 帐户 ID
  - **要求**: 必须是有效账户（校验和已验证）

**退货**:
- `Ok(())` - 管理员更改成功
- `Err(Error::Unauthorized)` - 来电者并非当前管理员

**发出的事件**:
- [`AdminChanged`](crate::AdminChanged) - 记录新旧管理员和呼叫者信息

**安全要求**:
- **访问控制**: 只有当前管理员才能调用
- **多重签名推荐**: 使用治理机制来管理生产变更
- **时间锁**: 出于安全原因考虑延迟

**天然气成本**: ~50,000 气体  
**例子**:
```rust
// 将管理员权限转移到新的多重签名钱包
contract.change_admin(new_multisig_wallet)?;
println!("Admin transferred successfully");
```

---

##### `set_oracle(oracle: AccountId) -> Result<(), Error>`

配置价格预言机合约地址。

**参数**:
- `oracle` (`AccountId`) - 预言机合约地址
  - **要求**: 必须部署 Oracle 合约

**退货**:
- `Ok(())` - Oracle配置成功
- `Err(Error::Unauthorized)` - 来电者不是管理员

**天然气成本**: ~30,000 气体  
**例子**:
```rust
// 部署后配置 Oracle
contract.set_oracle(oracle_contract_address)?;
```

---

##### `set_fee_manager(fee_manager: Option<AccountId>) -> Result<(), Error>`

配置或移除费用管理合同。

**参数**:
- `fee_manager` (`Option<AccountId>`) - 费用经理地址或 `None` 禁用

**退货**:
- `Ok(())` - 配置已更新
- `Err(Error::Unauthorized)` - 来电者不是管理员

**天然气成本**: ~30,000 气体  

---

##### `set_compliance_registry(registry: Option<AccountId>) -> Result<(), Error>`

配置或移除合规性注册表合约。

**参数**:
- `registry` (`Option<AccountId>`) - 合规注册地址或 `None`

**退货**:
- `Ok(())` - 配置已更新
- `Err(Error::Unauthorized)` - 来电者不是管理员

**天然气成本**: ~30,000 气体 

---

##### `update_valuation_from_oracle(property_id: u64) -> Result<(), Error>`

使用Oracle价格数据源更新房产估值。

**参数**:
- `property_id` (`u64`) - 要更新的属性 ID
  - **约束条件**: 必须存在于注册表中

**退货**:
- `Ok(())` - 估值更新成功
- `Err(Error::PropertyNotFound)` - 该房产不存在
- `Err(Error::OracleError)` - Oracle 调用失败
- `Err(Error::OracleError)` - Oracle 未配置

**发出的事件**:
- 属性元数据更新事件（间接）

**天然气成本**: ~75,000 天然气（交叉合约看涨期权）
**例子**:
```rust
// 出售前更新估价
contract.update_valuation_from_oracle(property_id)?;
let valuation = get_current_valuation(property_id);
```

---

##### `pause_contract(reason: String, duration_seconds: Option<u64>) -> Result<(), Error>`

出售前更新估价

**参数**:
- `reason` (`String`) - 人类可读的暂停原因
  - **Max Length**: 1024 人物
  - **例子**: `"Emergency maintenance - security audit"`
- `duration_seconds` (`Option<u64>`) - 可选的自动恢复延迟
  - **例子**: `Some(86400)` 24小时
  - **没有任何**: 需要手动简历

**退货**:
- `Ok(())` - 合约暂停成功
- `Err(Error::NotAuthorizedToPause)` - 来电者无权限
- `Err(Error::AlreadyPaused)` - 合同已经暂停

**发出的事件**:
- [`ContractPaused`](crate::ContractPaused) - 包括原因和自动恢复时间

**安全要求**:
- **访问控制**: 仅限管理员或暂停守护者
- **谨慎使用**: 仅紧急情况
- **沟通**: 公开宣布暂停

**天然气成本**: ~50,000 气体  
**例子**:
```rust
// 紧急暂停
contract.pause_contract(
    "Critical vulnerability discovered".to_string(),
    None // 需要手动简历
)?;
```

---

##### `emergency_pause(reason: String) -> Result<(), Error>`

立即暂停，不自动恢复（紧急情况）。

**参数**:
- `reason` (`String`) - 紧急原因

**退货**: 与相同 `pause_contract`  
**天然气成本**: ~50,000 气体  
**笔记**: 相当于 `pause_contract(reason, None)`

---

##### `try_auto_resume() -> Result<(), Error>`

如果自动恢复时间已过，则尝试恢复合同。

**参数**: 没有任何  
**退货**:
- `Ok(())` - 合同成功恢复
- `Err(Error::NotPaused)` - 合同未暂停
- `Err(Error::ResumeRequestNotFound)` - 没有有效的恢复请求

**发出的事件**:
- [`ContractResumed`](crate::ContractResumed)

**天然气成本**: ~30,000 气体  

---

## 错误处理指南

### 常见错误模式

#### 1. 授权失败

```rust
match contract.operation() {
    Ok(result) => process(result),
    Err(Error::Unauthorized) => {
        eprintln!("Access denied - check permissions");
        // 引导用户请求访问权限
    }
    Err(e) => handle_other_error(e),
}
```

#### 2. 合规失败

```rust
match contract.transfer_property(buyer, token_id) {
    Ok(_) => println!("Transfer complete"),
    Err(Error::NotCompliant) => {
        eprintln!("Buyer not compliant");
        eprintln!("Required: Complete KYC at https://kyc.propchain.io");
    }
    Err(e) => eprintln!("Error: {:?}", e),
}
```

#### 3. 验证失败

```rust
// 提交前进行预验证
fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), &'static str> {
    if metadata.location.is_empty() {
        return Err("Location required");
    }
    if metadata.valuation < 1000 {
        return Err("Minimum valuation $10");
    }
    Ok(())
}

// 然后提交
match validate_metadata(&metadata) {
    Ok(_) => contract.register_property(metadata)?,
    Err(e) => eprintln!("Invalid metadata: {}", e),
}
```

### 完整的错误参考

看 [API_ERROR_CODES.md](./API_ERROR_CODES.md) 提供所有错误类型的完整文档，包括：
- 触发条件
- 常见场景
- 恢复步骤
- 示例
- HTTP 等效项

---

## 集成示例

### 前端集成（React/TypeScript）

```typescript
import { useContract } from '@polkadot/react-hooks';

function RegisterPropertyForm() {
  const contract = useContract(CONTRACT_ADDRESS);
  
  const handleSubmit = async (metadata: PropertyMetadata) => {
    try {
      // 首先检查合规性
      const isCompliant = await contract.query.checkAccountCompliance(
        currentUser.address
      );
      
      if (!isCompliant) {
        throw new Error('Complete KYC first');
      }
      
      // 提交注册
      const tx = await contract.tx.registerProperty(metadata);
      await tx.signAndSend(currentUser.pair, ({ status, events }) => {
        if (status.isInBlock) {
          console.log('Transaction included in block');
          
          // 从事件中提取属性 ID
          const propertyRegistered = events.find(
            e => e.event.method === 'PropertyRegistered'
          );
          const propertyId = propertyRegistered?.event.data[0];
          console.log('Property ID:', propertyId.toString());
        }
      });
    } catch (error) {
      if (error.message.includes('NotCompliant')) {
        alert('Please complete KYC verification first');
      } else if (error.message.includes('InvalidMetadata')) {
        alert('Please check property details');
      } else {
        console.error('Registration failed:', error);
      }
    }
  };
  
  return (
    <form onSubmit={handleSubmit}>
      {/* 表单字段 */}
    </form>
  );
}
```

### 后端集成（Node.js）

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function registerProperty(metadata) {
  const api = await ApiPromise.create({
    provider: new WsProvider('wss://rpc.propchain.io')
  });
  
  // 查询当前状态
  const health = await api.query.propertyRegistry.healthCheck();
  if (!health.isHealthy) {
    throw new Error('Contract not healthy');
  }
  
  // 检查合规性
  const isCompliant = await api.query.complianceRegistry.isCompliant(
    userAddress
  );
  if (!isCompliant) {
    throw new Error('User not compliant');
  }
  
  // 提交交易
  const tx = api.tx.propertyRegistry.registerProperty(metadata);
  const hash = await tx.signAndSend(keypair);
  
  console.log('Transaction submitted:', hash.toHex());
  return hash;
}
```

### 智能合约集成

```rust
// 交叉合约看涨期权模式
use ink::env::call::FromAccountId;

fn integrate_with_property_registry(
    registry_addr: AccountId,
    metadata: PropertyMetadata
) -> Result<u64, Error> {
    let registry: ink::contract_ref!(PropertyRegistry) = 
        FromAccountId::from_account_id(registry_addr);
    
    // 调用注册表方法
    let property_id = registry.register_property(metadata)?;
    
    Ok(property_id)
}
```

---

## 活动参考

### 要监控的关键事件

#### `PropertyRegistered`

当注册新房产时发出。

**索引字段** (可过滤的):
- `property_id: u64`
- `owner: AccountId`

**数据字段**:
- `location: String`
- `size: u64`
- `valuation: u128`
- `timestamp: u64`
- `block_number: u32`
- `transaction_hash: Hash`

**使用案例**:
- 指数财产所有权
- 触发链下工作流程
- 更新分析仪表板

---

#### `PropertyTransferred`

当房产所有权发生变更时会排放。

**索引字段**:
- `property_id: u64`
- `from: AccountId`
- `to: AccountId`

**使用案例**:
- 更新所有权记录
- 计算转让税
- 跟踪投资组合

---

#### `EscrowCreated` / `EscrowReleased`

跟踪托管资金的生命周期，确保资金安全转移。

**使用案例**:
- 监控交易进度
- 检测卡住的托管账户
- 计算托管费用

---

## 气体优化技巧

### 1. 批量操作

```rust
// ❌ 费用高昂：多次交易
for property in properties {
    contract.register_property(property)?;
}

// ✅ 更便宜：单笔交易
contract.batch_register_properties(properties)?;
```

### 2. 预验证

```rust
// 先进行链下验证，避免浪费 gas
if !validate_metadata_locally(&metadata) {
    return Err("Invalid metadata"); // 不提交可以节省汽油
}
```

### 3. 高效查询

```rust
// ❌ 成本高昂：循环查询
for id in property_ids {
    let prop = contract.get_property(id)?; // Multiple calls
}

// ✅ 更佳方案：如果可用，请使用批量查询。
let props = contract.get_properties_batch(property_ids)?; // 单次通话
```

---

## 测试指南

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_property() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_test_metadata();
        
        let result = contract.register_property(metadata);
        assert!(result.is_ok());
        
        let property_id = result.unwrap();
        assert!(property_id > 0);
    }
    
    #[test]
    fn test_unauthorized_admin_change() {
        let mut contract = PropertyRegistry::new();
        let unauthorized_account = AccountId::from([1u8; 32]);
        
        // 将呼叫者设置为未经授权的帐户
        set_caller(unauthorized_account);
        
        let result = contract.change_admin(AccountId::from([2u8; 32]));
        assert!(matches!(result, Err(Error::Unauthorized)));
    }
}
```

### 集成测试

```rust
#[ink_e2e::test]
async fn test_full_property_lifecycle(mut client: ink_e2e::Client<C, E>) {
    // 设置
    let mut builder = build_contract!("propchain_contracts", "PropertyRegistry");
    let contract_id = client.instantiate("propchain_contracts", &bob, 0, &mut builder).await?;
    
    // 登记财产
    let metadata = create_metadata();
    let register_msg = propchain_contracts::Message::RegisterProperty { metadata };
    let result = client.call(&bob, register_msg, &mut storage()).await?;
    
    // 核实
    assert!(result.return_value().is_ok());
}
```

---

## 相关文档

- **[API Documentation Standards](./API_DOCUMENTATION_STANDARDS.md)** - 我们如何编写 API 文档
- **[API Error Codes](./API_ERROR_CODES.md)** - 综合错误参考
- **[Architecture Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)** - 系统上下文
- **[Integration Guide](./integration.md)** - 一般集成模式
- **[Troubleshooting FAQ](./troubleshooting-faq.md)** - 常见问题

---

## 获取帮助

### 资源

- **GitHub Issues**: 用于报告错误或请求功能（响应时间：24-48小时）。
- **Discord**: 实时开发者支持（响应时间：< 1小时）。
- **Stack Overflow**: 技术问答，请使用 `propchain` 标签。
- **Documentation**: 完整文档请访问 docs.propchain.io

### 支持渠道

| 问题类型 | 最佳频道 | 响应时间 |
|------------|--------------|---------------|
| 错误报告 | GitHub 问题 | 24-48 小时 |
| 集成帮助 | 不和谐 #dev-support | < 1 小时 |
| 安全问题 | security@propchain.io | 即时 |
| 一般问题 | 堆栈溢出 | 2-24 小时 |

---

**最后更新**: April 22, 2026  
**版本**: 1.0.0  
**维护者**: PropChain 开发团队
