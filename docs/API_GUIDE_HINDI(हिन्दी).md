# PropChain API दस्तावेज़ीकरण गाइड

## अवलोकन (Overview)

यह गाइड डेवलपर्स को PropChain स्मार्ट कॉन्ट्रैक्ट्स के साथ एकीकृत करने के लिए पूर्ण और अच्छी तरह से प्रलेखित API प्रदान करती है। यह [API_DOCUMENTATION_STANDARDS.md](./API_DOCUMENTATION_STANDARDS.md) में परिभाषित मानकों का पालन करती है और इसमें [API_ERROR_CODES.md](./API_ERROR_CODES.md) से व्यापक त्रुटि दस्तावेज़ीकरण शामिल है।

---

## त्वरित शुरुआत (Quick Start)

### 1. आपको जो चाहिए वो ढूंढें

**उपयोग के मामले के अनुसार (By Use Case)**:
- **संपत्ति पंजीकृत करें (Register Property)**: [`register_property`](#register_property) देखें
- **स्वामित्व हस्तांतरित करें (Transfer Ownership)**: [`transfer_property`](#transfer_property) देखें
- **अनुपालन की जाँच करें (Check Compliance)**: [`check_account_compliance`](#check_account_compliance) देखें
- **एस्क्रो बनाएं (Create Escrow)**: [Escrow Contract](#escrow-contract) देखें
- **मूल्यांकन प्राप्त करें (Get Valuation)**: [Oracle Contract](#oracle-contract) देखें

**भूमिका के अनुसार (By Role)**:
- **फ्रंटएंड डेवलपर**: उदाहरणों और बुनियादी संचालन के साथ शुरू करें
- **बैकएंड डेवलपर**: इवेंट्स और स्टेट क्वेरीज़ पर ध्यान दें
- **स्मार्ट कॉन्ट्रैक्ट डेवलपर**: एकीकरण पैटर्न और क्रॉस-कॉन्ट्रैक्ट कॉल्स की समीक्षा करें
- **लेखा परीक्षक**: त्रुटि प्रबंधन और सुरक्षा आवश्यकताओं का अध्ययन करें

---

## मुख्य API संदर्भ (Core API Reference)

### संपत्ति रजिस्ट्री अनुबंध (Property Registry Contract)

संपत्ति प्रबंधन और स्वामित्व ट्रैकिंग के लिए मुख्य अनुबंध।

#### निर्माता (Constructor)

##### `new()`

एक नया PropertyRegistry कॉन्ट्रैक्ट उदाहरण बनाता और प्रारंभ करता है।

**प्रलेखन**: सोर्स कोड में विस्तृत रस्टडॉक देखें
**उदाहरण**:
```rust
// स्वचालित रूप से तैनात - किसी मैन्युअल कॉल की आवश्यकता नहीं है
let contract = PropertyRegistry::new();
assert_eq!(contract.version(), 1);
```

---

#### केवल पढ़ने योग्य फ़ंक्शन (व्यू मेथड्स)

ये फ़ंक्शन स्थिति को संशोधित नहीं करते हैं और इन्हें कॉल करने के लिए स्वतंत्र हैं।

##### `version() -> u32`

कॉन्ट्रैक्ट संस्करण संख्या देता है (वर्तमान में 1)।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `u32` - संस्करण संख्या (वर्तमान में 1)
**गैस लागत**: ~500 गैस  
**उदाहरण**:
```rust
let version = contract.version();
if version >= 2 {
    // नई सुविधाओं का प्रयोग करें
}
```

---

##### `admin() -> AccountId`

एडमिन अकाउंट का एड्रेस देता है।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `AccountId` - व्यवस्थापक का सबस्ट्रेट खाता
**गैस लागत**: ~500 गैस  
**उदाहरण**:
```rust
let admin = contract.admin();
println!("Contract admin: {:?}", admin);
```

---

##### `health_check() -> HealthStatus`

निगरानी के लिए व्यापक स्वास्थ्य स्थिति (संपत्तियों की कुल संख्या, सक्रिय एस्क्रो और ओरेकल स्थिति सहित)।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: [`HealthStatus`](crate::HealthStatus) इसके साथ संरचना:
- `is_healthy: bool` - समग्र स्वास्थ्य ध्वज
- `is_paused: bool` - स्थिति रोकें
- `contract_version: u32` - संस्करण क्रमांक
- `property_count: u64` - कुल संपत्ति
- `escrow_count: u64` - सक्रिय एस्क्रो
- `has_oracle: bool` - ओरेकल कॉन्फ़िगर किया गया
- `has_compliance_registry: bool` - अनुपालन कॉन्फ़िगर किया गया
- `has_fee_manager: bool` - शुल्क प्रबंधक कॉन्फ़िगर किया गया
- `block_number: u32` - वर्तमान ब्लॉक
- `timestamp: u64` - वर्तमान टाइमस्टैम्प

**गैस लागत**: ~2,000 गैस  
**उदाहरण**:
```rust
let health = contract.health_check();
if !health.is_healthy {
    alert_admins("Contract issues detected!");
}
println!("Properties: {}", health.property_count);
```

---

##### `ping() -> bool`

सरल लाइवनेस जांच।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `bool` - हमेशा लौटता है `true` यदि अनुबंध उत्तरदायी है 
**गैस लागत**: ~500 गैस  
**Use Case**: सुनिश्चित करें कि अनुबंध लागू और चालू है।

---

##### `dependencies_healthy() -> bool`

यह जांच करता है कि सभी महत्वपूर्ण निर्भरताएं कॉन्फ़िगर की गई हैं या नहीं।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `bool` - `true` यदि ऑरेकल, कंप्लायंस और फी मैनेजर सभी कॉन्फ़िगर किए गए हैं
**गैस लागत**: ~1,000 गैस  
**उदाहरण**:
```rust
if contract.dependencies_healthy() {
    println!("All systems operational");
} else {
    println!("Some dependencies not configured");
}
```

---

##### `oracle() -> Option<AccountId>`

यह ऑरेकल कॉन्ट्रैक्ट का पता लौटाता है।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `Option<AccountId>` - कॉन्फ़िगर किए जाने पर ऑरेकल पता
**गैस लागत**: ~500 गैस

---

##### `get_fee_manager() -> Option<AccountId>`

यह शुल्क प्रबंधक अनुबंध का पता लौटाता है।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `Option<AccountId>` - यदि कॉन्फ़िगर किया गया हो तो शुल्क प्रबंधक का पता 
**गैस लागत**: ~500 गैस  

---

##### `get_compliance_registry() -> Option<AccountId>`

यह अनुपालन रजिस्ट्री अनुबंध का पता लौटाता है।

**पैरामीटर**: कोई नहीं  
**रिटर्न**: `Option<AccountId>` - यदि कॉन्फ़िगर किया गया हो तो अनुपालन रजिस्ट्री पता
**गैस लागत**: ~500 गैस  

---

##### `check_account_compliance(account: AccountId) -> Result<bool, Error>`

यह जाँचता है कि क्या कोई खाता अनुपालन आवश्यकताओं को पूरा करता है।

**पैरामीटर**:
- `account` (`AccountId`) - खाता जांचना है

**रिटर्न**:
- `Ok(bool)` - `true` यदि अनुपालन हो, `false` अन्यथा
- `Err(Error)` - यदि अनुपालन जांच तकनीकी रूप से विफल हो जाती है

**त्रुटियाँ**:
- [`Error::ComplianceCheckFailed`](./API_ERROR_CODES.md#error-compliancecheckfailed) - रजिस्ट्री कॉल विफल रही
- [`Error::OracleError`](./API_ERROR_CODES.md#error-oracleerror) - क्रॉस-कॉन्ट्रैक्ट कॉल विफलता

**गैस लागत**: ~5,000 गैस (इसमें क्रॉस-कॉन्ट्रैक्ट कॉल शामिल है)
**उदाहरण**:
```rust
match contract.check_account_compliance(buyer_account) {
    Ok(true) => println!("Account is compliant"),
    Ok(false) => println!("Account NOT compliant - needs KYC"),
    Err(e) => eprintln!("Compliance check error: {:?}", e),
}
```

---

##### `get_dynamic_fee(operation: FeeOperation) -> u128`

किसी विशिष्ट ऑपरेशन के लिए गतिशील शुल्क लौटाता है।

**पैरामीटर**:
- `operation` (`FeeOperation`) - ऑपरेशन का प्रकार

**रिटर्न**:
- `u128` - शुल्क राशि सबसे छोटी मुद्रा इकाई (सेंट) में

**गैस लागत**: ~3,000 गैस  
**उदाहरण**:
```rust
let fee = contract.get_dynamic_fee(FeeOperation::PropertyTransfer);
println!("Transfer fee: {} cents", fee);
```

---

#### स्थिति-परिवर्तनकारी फ़ंक्शन (लेन-देन)

ये फ़ंक्शन अनुबंध की स्थिति को संशोधित करते हैं और गैस की आवश्यकता होती है।

##### `change_admin(new_admin: AccountId) -> Result<(), Error>`

प्रशासनिक विशेषाधिकारों को एक नए खाते में स्थानांतरित करता है।

**पैरामीटर**:
- `new_admin` (`AccountId`) - व्यवस्थापक विशेषाधिकार प्राप्त करने के लिए खाता
  - **प्रारूप**: 32-बाइट सबस्ट्रेट खाता आईडी
  - **आवश्यकताएं**: वैध खाता होना चाहिए (चेकसम सत्यापित)।

**रिटर्न**:
- `Ok(())` - एडमिन सफलतापूर्वक बदला गया
- `Err(Error::Unauthorized)` - कॉल करने वाला वर्तमान एडमिन नहीं है

**उत्सर्जित घटनाएँ**:
- [`AdminChanged`](crate::AdminChanged) - पुराने/नए एडमिन और कॉलर के लॉग

**सुरक्षा आवश्यकताएँ**:
- **अभिगम नियंत्रण**: केवल वर्तमान व्यवस्थापक ही कॉल कर सकता है
- **मल्टी-सिग अनुशंसित**: उत्पादन परिवर्तनों के लिए शासन का उपयोग करें
- **Timelock**: सुरक्षा के लिए देरी पर विचार करें

**गैस लागत**: ~50,000 गैस  
**उदाहरण**:
```rust
// एडमिन को नए मल्टीसिग वॉलेट में ट्रांसफर करें
contract.change_admin(new_multisig_wallet)?;
println!("Admin transferred successfully");
```

---

##### `set_oracle(oracle: AccountId) -> Result<(), Error>`

यह ओरेकल अनुबंध के मूल्य पते को कॉन्फ़िगर करता है।

**पैरामीटर**:
- `oracle` (`AccountId`) - ओरेकल अनुबंध पता
  - **आवश्यकताएं**: ओरेकल अनुबंध को तैनात करना आवश्यक है

**रिटर्न**:
- `Ok(())` - Oracle सफलतापूर्वक कॉन्फ़िगर किया गया
- `Err(Error::Unauthorized)` - कॉल करने वाला एडमिन नहीं है

**गैस लागत**: ~30,000 गैस  
**उदाहरण**:
```rust
// तैनाती के बाद ओरेकल को कॉन्फ़िगर करें
contract.set_oracle(oracle_contract_address)?;
```

---

##### `set_fee_manager(fee_manager: Option<AccountId>) -> Result<(), Error>`

शुल्क प्रबंधक अनुबंध को कॉन्फ़िगर या हटाता है।

**पैरामीटर**:
- `fee_manager` (`Option<AccountId>`) - शुल्क प्रबंधक का पता या `None` अक्षम करना

**रिटर्न**:
- `Ok(())` - कॉन्फ़िगरेशन अपडेट किया गया
- `Err(Error::Unauthorized)` - कॉल करने वाला एडमिन नहीं है

**गैस लागत**: ~30,000 गैस  

---

##### `set_compliance_registry(registry: Option<AccountId>) -> Result<(), Error>`

अनुपालन रजिस्ट्री अनुबंध को कॉन्फ़िगर या हटाता है।

**पैरामीटर**:
- `registry` (`Option<AccountId>`) - अनुपालन रजिस्ट्री पता या `None`

**रिटर्न**:
- `Ok(())` - कॉन्फ़िगरेशन अपडेट किया गया
- `Err(Error::Unauthorized)` - कॉल करने वाला एडमिन नहीं है

**गैस लागत**: ~30,000 गैस  

---

##### `update_valuation_from_oracle(property_id: u64) -> Result<(), Error>`

ओरेकल प्राइस फीड का उपयोग करके संपत्ति के मूल्यांकन को अपडेट करता है।

**पैरामीटर**:
- `property_id` (`u64`) - अपडेट की जाने वाली प्रॉपर्टी की आईडी
  - **प्रतिबंध**: रजिस्ट्री में मौजूद होना चाहिए

**रिटर्न**:
- `Ok(())` - मूल्यांकन सफलतापूर्वक अपडेट हो गया है
- `Err(Error::PropertyNotFound)` - संपत्ति मौजूद नहीं है
- `Err(Error::OracleError)` - ओरेकल कॉल विफल रही
- `Err(Error::OracleError)` - Oracle कॉन्फ़िगर नहीं किया गया

**उत्सर्जित घटनाएँ**:
- संपत्ति मेटाडेटा अद्यतन घटना (अप्रत्यक्ष रूप से)

**गैस लागत**: ~75,000 गैस (क्रॉस-कॉन्ट्रैक्ट कॉल)
**उदाहरण**:
```rust
// बिक्री से पहले मूल्यांकन अद्यतन करें
contract.update_valuation_from_oracle(property_id)?;
let valuation = get_current_valuation(property_id);
```

---

##### `pause_contract(reason: String, duration_seconds: Option<u64>) -> Result<(), Error>`

सभी गैर-महत्वपूर्ण अनुबंध कार्यों को रोक देता है।

**पैरामीटर**:
- `reason` (`String`) - मानव-पठनीय विराम कारण
  - **अधिकतम लंबाई**: 1024 अक्षर
  - **उदाहरण**: `"Emergency maintenance - security audit"`
- `duration_seconds` (`Option<u64>`) - वैकल्पिक ऑटो-रिज्यूम विलंब
  - **उदाहरण**: `Some(86400)` 24 घंटे के लिए
  - **कोई नहीं**: मैन्युअल बायोडाटा आवश्यक है

**रिटर्न**:
- `Ok(())` - अनुबंध सफलतापूर्वक रोका गया
- `Err(Error::NotAuthorizedToPause)` - कॉल करने वाले के पास अनुमति नहीं है
- `Err(Error::AlreadyPaused)` - अनुबंध पहले ही रुका हुआ है

**उत्सर्जित घटनाएँ**:
- [`ContractPaused`](crate::ContractPaused) - इसमें कारण और ऑटो-रिज्यूम का समय शामिल है।

**सुरक्षा आवश्यकताएँ**:
- **अभिगम नियंत्रण**: केवल व्यवस्थापक या रोके रखने वाले संरक्षक ही इसका उपयोग कर सकते हैं।
- **संयम से प्रयोग करें**: केवल आपातकालीन स्थितियाँ
- **संचार**: सार्वजनिक रूप से विराम की घोषणा करें

**गैस लागत**: ~50,000 गैस  
**उदाहरण**:
```rust
// आपातकालीन विराम
contract.pause_contract(
    "Critical vulnerability discovered".to_string(),
    None // मैन्युअल बायोडाटा आवश्यक है
)?;
```

---

##### `emergency_pause(reason: String) -> Result<(), Error>`

तत्काल विराम, स्वतः पुनः आरंभ नहीं (अत्यंत गंभीर आपात स्थिति)।

**पैरामीटर**:
- `reason` (`String`) - आपातकालीन कारण

**रिटर्न**: `pause_contract` के समान
**गैस लागत**: ~50,000 गैस  
**Note**: `pause_contract(reason, None)` के बराबर

---

##### `try_auto_resume() -> Result<(), Error>`

यदि स्वतः पुनः आरंभ होने का समय बीत चुका है तो अनुबंध को पुनः आरंभ करने का प्रयास किया जाएगा।

**पैरामीटर**: कोई नहीं  
**रिटर्न**:
- `Ok(())` - अनुबंध सफलतापूर्वक फिर से शुरू हुआ
- `Err(Error::NotPaused)` - अनुबंध नहीं रोका गया
- `Err(Error::ResumeRequestNotFound)` - कोई सक्रिय बायोडाटा अनुरोध नहीं

**उत्सर्जित घटनाएँ**:
- [`ContractResumed`](crate::ContractResumed)

**गैस लागत**: ~30,000 गैस  

---

## त्रुटि प्रबंधन मार्गदर्शिका (Error Handling Guide)

### सामान्य त्रुटि पैटर्न (Common Error Patterns)

#### 1. प्राधिकरण विफलताएँ (Authorization Failures)

```rust
match contract.operation() {
    Ok(result) => process(result),
    Err(Error::Unauthorized) => {
        eprintln!("Access denied - check permissions");
        // उपयोगकर्ता को एक्सेस का अनुरोध करने के लिए मार्गदर्शन करें
    }
    Err(e) => handle_other_error(e),
}
```

#### 2. अनुपालन विफलताएँ (Compliance Failures)

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

#### 3. सत्यापन विफलताएँ (Validation Failures)

```rust
// सबमिशन से पहले पूर्व-सत्यापन करें
fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), &'static str> {
    if metadata.location.is_empty() {
        return Err("Location required");
    }
    if metadata.valuation < 1000 {
        return Err("Minimum valuation $10");
    }
    Ok(())
}

// फिर सबमिट करें
match validate_metadata(&metadata) {
    Ok(_) => contract.register_property(metadata)?,
    Err(e) => eprintln!("Invalid metadata: {}", e),
}
```

### पूर्ण त्रुटि संदर्भ (Complete Error Reference)

[API_ERROR_CODES.md](./API_ERROR_CODES.md) देखना सभी प्रकार की त्रुटियों के व्यापक दस्तावेज़ीकरण के लिए, जिसमें शामिल हैं:
- ट्रिगर स्थितियाँ
- सामान्य परिदृश्य
- पुनर्प्राप्ति चरण
- उदाहरण
- HTTP समकक्ष

---

## एकीकरण उदाहरण (Integration Examples)

### फ्रंटएंड इंटीग्रेशन (रिएक्ट/टाइपस्क्रिप्ट)

```typescript
import { useContract } from '@polkadot/react-hooks';

function RegisterPropertyForm() {
  const contract = useContract(CONTRACT_ADDRESS);
  
  const handleSubmit = async (metadata: PropertyMetadata) => {
    try {
      // पहले अनुपालन की जाँच करें
      const isCompliant = await contract.query.checkAccountCompliance(
        currentUser.address
      );
      
      if (!isCompliant) {
        throw new Error('Complete KYC first');
      }
      
      // पंजीकरण सबमिट करें
      const tx = await contract.tx.registerProperty(metadata);
      await tx.signAndSend(currentUser.pair, ({ status, events }) => {
        if (status.isInBlock) {
          console.log('Transaction included in block');
          
          // इवेंट्स से प्रॉपर्टी आईडी निकालें
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
      {/* प्रपत्र फ़ील्ड */}
    </form>
  );
}
```

### बैकएंड इंटीग्रेशन (नोड.जेएस)

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function registerProperty(metadata) {
  const api = await ApiPromise.create({
    provider: new WsProvider('wss://rpc.propchain.io')
  });
  
  // वर्तमान स्थिति पूछें
  const health = await api.query.propertyRegistry.healthCheck();
  if (!health.isHealthy) {
    throw new Error('Contract not healthy');
  }
  
  // अनुपालन की जाँच करें
  const isCompliant = await api.query.complianceRegistry.isCompliant(
    userAddress
  );
  if (!isCompliant) {
    throw new Error('User not compliant');
  }
  
  // लेनदेन सबमिट करें
  const tx = api.tx.propertyRegistry.registerProperty(metadata);
  const hash = await tx.signAndSend(keypair);
  
  console.log('Transaction submitted:', hash.toHex());
  return hash;
}
```

### स्मार्ट अनुबंध एकीकरण (Smart Contract Integration)

```rust
// क्रॉस-कॉन्ट्रैक्ट कॉल पैटर्न
use ink::env::call::FromAccountId;

fn integrate_with_property_registry(
    registry_addr: AccountId,
    metadata: PropertyMetadata
) -> Result<u64, Error> {
    let registry: ink::contract_ref!(PropertyRegistry) = 
        FromAccountId::from_account_id(registry_addr);
    
    // रजिस्ट्री विधि को कॉल करें
    let property_id = registry.register_property(metadata)?;
    
    Ok(property_id)
}
```

---

## घटना संदर्भ (Events Reference)

### निगरानी के लिए प्रमुख घटनाएँ (Key Events to Monitor)

#### `PropertyRegistered`

जब कोई नई संपत्ति पंजीकृत होती है तो यह संदेश उत्सर्जित होता है।

**अनुक्रमित फ़ील्ड** (filterable):
- `property_id: u64`
- `owner: AccountId`

**डेटा फ़ील्ड**:
- `location: String`
- `size: u64`
- `valuation: u128`
- `timestamp: u64`
- `block_number: u32`
- `transaction_hash: Hash`

**मामलों का प्रयोग करें**:
- सूचकांक संपत्ति स्वामित्व
- ऑफ-चेन वर्कफ़्लो को ट्रिगर करें
- एनालिटिक्स डैशबोर्ड अपडेट करें

---

#### `PropertyTransferred`

संपत्ति के स्वामित्व में परिवर्तन होने पर उत्सर्जित होता है।

**अनुक्रमित फ़ील्ड**:
- `property_id: u64`
- `from: AccountId`
- `to: AccountId`

**मामलों का प्रयोग करें**:
- स्वामित्व रिकॉर्ड अद्यतन करें
- स्थानांतरण करों की गणना करें
- निवेश पोर्टफोलियो को ट्रैक करें

---

#### `EscrowCreated` / `EscrowReleased`

सुरक्षित हस्तांतरण के लिए एस्क्रो की पूरी प्रक्रिया पर नज़र रखें।

**मामलों का प्रयोग करें**:
- लेन-देन की प्रगति की निगरानी करें
- अटके हुए एस्क्रो का पता लगाएं
- एस्क्रो शुल्क की गणना करें

---

## गैस अनुकूलन युक्तियाँ (Gas Optimization Tips)

### 1. बैच संचालन (Batch Operations)

```rust
// ❌ महंगा: एकाधिक लेनदेन
for property in properties {
    contract.register_property(property)?;
}

// ✅ सस्ता: एकल बैच लेनदेन
contract.batch_register_properties(properties)?;
```

### 2. पूर्व सत्यापन (Pre-validation)

```rust
// ईंधन की बर्बादी से बचने के लिए पहले ऑफ-चेन सत्यापन करें।
if !validate_metadata_locally(&metadata) {
    return Err("Invalid metadata"); // सबमिट न करके गैस बचाएं
}
```

### 3. कुशल क्वेरीज़ (Efficient Queries)

```rust
// ❌ महंगा: लूप में क्वेरी
for id in property_ids {
    let prop = contract.get_property(id)?; // एकाधिक कॉल
}

// ✅ बेहतर होगा: यदि उपलब्ध हो तो बैच क्वेरी का उपयोग करें
let props = contract.get_properties_batch(property_ids)?; // एकल कॉल
```

---

## परीक्षण मार्गदर्शिका (Testing Guide)

### यूनिट परीक्षण (Unit Tests)

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
        
        // कॉलर को अनधिकृत खाते पर सेट करें
        set_caller(unauthorized_account);
        
        let result = contract.change_admin(AccountId::from([2u8; 32]));
        assert!(matches!(result, Err(Error::Unauthorized)));
    }
}
```

### एकीकरण परीक्षण (Integration Tests)

```rust
#[ink_e2e::test]
async fn test_full_property_lifecycle(mut client: ink_e2e::Client<C, E>) {
    // स्थापित करना
    let mut builder = build_contract!("propchain_contracts", "PropertyRegistry");
    let contract_id = client.instantiate("propchain_contracts", &bob, 0, &mut builder).await?;
    
    // संपत्ति पंजीकृत करें
    let metadata = create_metadata();
    let register_msg = propchain_contracts::Message::RegisterProperty { metadata };
    let result = client.call(&bob, register_msg, &mut storage()).await?;
    
    // सत्यापित करें
    assert!(result.return_value().is_ok());
}
```

---

## संबंधित दस्तावेज़ीकरण (Related Documentation)

- **[API Documentation Standards](./API_DOCUMENTATION_STANDARDS.md)** - हम API को कैसे दस्तावेज़ित करते हैं
- **[API Error Codes](./API_ERROR_CODES.md)** - व्यापक त्रुटि संदर्भ
- **[Architecture Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)** - सिस्टम संदर्भ
- **[Integration Guide](./integration.md)** - सामान्य एकीकरण पैटर्न
- **[Troubleshooting FAQ](./troubleshooting-faq.md)** - सामान्य मुद्दे

---

## सहायता प्राप्त करना (Getting Help)

### संसाधन (Resources)

- **GitHub Issues**: बग की रिपोर्ट करने या सुविधाओं का अनुरोध करने के लिए (प्रतिक्रिया समय: 24-48 घंटे)।
- **Discord**: रीयल-टाइम डेवलपर समर्थन (प्रतिक्रिया समय: < 1 घंटा)।
- **Stack Overflow**: तकनीकी प्रश्न और उत्तर (टैग: `propchain`)।
- **Documentation**: docs.propchain.io पर संपूर्ण दस्तावेज़ उपलब्ध हैं।

### समर्थन चैनल (Support Channels)

| विषय वर्ग | सर्वोत्तम चैनल | प्रतिक्रिया समय |
|------------|--------------|---------------|
| दोष रिपोर्ट | गिटहब मुद्दे | 24-48 घंटे |
| एकीकरण सहायता | कलह #dev-support | < 1 घंटा |
| सुरक्षा समस्याएं | security@propchain.io | तुरंत |
| सामान्य प्रश्न | स्टैक ओवरफ़्लो | 2-24 घंटे |

---

**आखरी अपडेट**: April 22, 2026  
**संस्करण**: 1.0.0  
**संधृत द्वारा**: प्रॉपचेन विकास टीम
