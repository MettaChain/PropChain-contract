# Guía de Documentación de la API de PropChain

## Descripción General

Esta guía proporciona a los desarrolladores API completas y bien documentadas para la integración con los contratos inteligentes de PropChain. Sigue los estándares definidos en [API_DOCUMENTATION_STANDARDS.md](./API_DOCUMENTATION_STANDARDS.md) e incluye documentación exhaustiva de errores de [API_ERROR_CODES.md](./API_ERROR_CODES.md).

---

## Inicio Rápido

### 1. Encuentra lo que necesitas

**Por Caso de Uso**:
- **Registrar Propiedad (Register Property)**: Consulte [`register_property`](#register_property)
- **Transferir Propiedad (Transfer Ownership)**: Consulte [`transfer_property`](#transfer_property)
- **Comprobar Cumplimiento (Check Compliance)**: Consulte [`check_account_compliance`](#check_account_compliance)
- **Crear depósito de garantía (Create Escrow)**: Consulte [Escrow Contract](#escrow-contract)
- **Obtener valoración (Get Valuation)**: Consulte [Oracle Contract](#oracle-contract)

**Por Rol**:
- **Desarrollador Frontend**: Comience con los ejemplos y operaciones básicas
- **Desarrollador Backend**: Concéntrese en eventos y consultas de estado
- **Desarrollador de Contratos Inteligentes**: Revise los patrones de integración y las llamadas entre contratos
- **Auditor**: Estudiar el manejo de errores y los requisitos de seguridad.

---

## Referencia de la API Principal

### Contrato de Registro de la Propiedad

El contrato principal para la gestión de propiedades y el seguimiento de la propiedad.

#### Constructor

##### `new()`

Crea e inicializa una nueva instancia del contrato PropertyRegistry.

**Documentación**: Consulte la documentación detallada de Rust en el código fuente.
**Ejemplo**:
```rust
// Se implementa automáticamente, sin necesidad de realizar ninguna llamada manual
let contract = PropertyRegistry::new();
assert_eq!(contract.version(), 1);
```

---

#### Funciones de solo lectura (métodos de visualización)

Estas funciones no modifican el estado y se pueden llamar libremente.

##### `version() -> u32`

Devuelve el número de versión del contrato (actualmente 1).

**Parámetros**: Ninguno  
**Devoluciones**: `u32` - Número de versión (actualmente 1)
**Costo de gasolina**: ~500 gas  
**Ejemplo**:
```rust
let version = contract.version();
if version >= 2 {
    // Utilice nuevas funciones
}
```

---

##### `admin() -> AccountId`

Devuelve la dirección de la cuenta del administrador.

**Parámetros**: Ninguno  
**Devoluciones**: `AccountId` - Cuenta de Substrate del administrador
**Costo de gasolina**: ~500 gas  
**Ejemplo**:
```rust
let admin = contract.admin();
println!("Contract admin: {:?}", admin);
```

---

##### `health_check() -> HealthStatus`

Estado de salud integral para monitorización (incluye recuento de propiedades, depósitos en garantía activos y estado del oráculo).

**Parámetros**: Ninguno  
**Devoluciones**: [`HealthStatus`](crate::HealthStatus) estructura con:
- `is_healthy: bool` - Bandera de salud general
- `is_paused: bool` - Estado de pausa
- `contract_version: u32` - Número de versión
- `property_count: u64` - Propiedades totales
- `escrow_count: u64` - Custodias activas
- `has_oracle: bool` - Oracle configurado
- `has_compliance_registry: bool` - Cumplimiento configurado
- `has_fee_manager: bool` - Administrador de tarifas configurado
- `block_number: u32` - bloque actual
- `timestamp: u64` - Marca de tiempo actual

**Costo de gasolina**: ~2,000 gas  
**Ejemplo**:
```rust
let health = contract.health_check();
if !health.is_healthy {
    alert_admins("Contract issues detected!");
}
println!("Properties: {}", health.property_count);
```

---

##### `ping() -> bool`

Comprobación sencilla de si está en directo.

**Parámetros**: Ninguno  
**Devoluciones**: `bool` - Siempre regresa `true` si el contrato es receptivo
**Costo de gasolina**: ~500 gas  
**Use Case**: Verificar que el contrato esté implementado y en funcionamiento.

---

##### `dependencies_healthy() -> bool`

Comprueba si todas las dependencias críticas están configuradas.

**Parámetros**: Ninguno  
**Devoluciones**: `bool` - `true` si Oracle, Cumplimiento y Administrador de tarifas están configurados
**Costo de gasolina**: ~1,000 gas  
**Ejemplo**:
```rust
if contract.dependencies_healthy() {
    println!("All systems operational");
} else {
    println!("Some dependencies not configured");
}
```

---

##### `oracle() -> Option<AccountId>`

Devuelve la dirección del contrato de Oracle.

**Parámetros**: Ninguno  
**Devoluciones**: `Option<AccountId>` - Dirección de Oracle si está configurado  
**Costo de gasolina**: ~500 gas  

---

##### `get_fee_manager() -> Option<AccountId>`

Devuelve la dirección del contrato del gestor de tarifas.

**Parámetros**: Ninguno  
**Devoluciones**: `Option<AccountId>` - Dirección del administrador de tarifas si está configurado
**Costo de gasolina**: ~500 gas  

---

##### `get_compliance_registry() -> Option<AccountId>`

Devuelve la dirección del contrato del registro de cumplimiento.

**Parámetros**: Ninguno  
**Devoluciones**: `Option<AccountId>` - Dirección del registro de cumplimiento si está configurada
**Costo de gasolina**: ~500 gas  

---

##### `check_account_compliance(account: AccountId) -> Result<bool, Error>`

Comprueba si una cuenta cumple con los requisitos normativos/de cumplimiento.

**Parámetros**:
- `account` (`AccountId`) - Cuenta para comprobar

**Devoluciones**:
- `Ok(bool)` - `true` si cumple, `false` de lo contrario
- `Err(Error)` - Si la verificación de cumplimiento falla técnicamente

**Errores**:
- [`Error::ComplianceCheckFailed`](./API_ERROR_CODES.md#error-compliancecheckfailed) - La llamada al registro falló
- [`Error::OracleError`](./API_ERROR_CODES.md#error-oracleerror) - Fallo en la llamada entre contratos

**Costo de gasolina**: ~5,000 gas (incluye llamadas entre contratos)
**Ejemplo**:
```rust
match contract.check_account_compliance(buyer_account) {
    Ok(true) => println!("Account is compliant"),
    Ok(false) => println!("Account NOT compliant - needs KYC"),
    Err(e) => eprintln!("Compliance check error: {:?}", e),
}
```

---

##### `get_dynamic_fee(operation: FeeOperation) -> u128`

Devuelve la tarifa dinámica para una operación específica.

**Parámetros**:
- `operation` (`FeeOperation`) - Tipo de operación

**Devoluciones**:
- `u128` - Importe de la comisión en la unidad monetaria más pequeña (centavos)

**Costo de gasolina**: ~3,000 gas  
**Ejemplo**:
```rust
let fee = contract.get_dynamic_fee(FeeOperation::PropertyTransfer);
println!("Transfer fee: {} cents", fee);
```

---

#### Funciones que modifican el estado (Transacciones)

Estas funciones modifican el estado del contrato y requieren gas.

##### `change_admin(new_admin: AccountId) -> Result<(), Error>`

Transfiere los privilegios de administrador a una nueva cuenta.

**Parámetros**:
- `new_admin` (`AccountId`) - Cuenta para recibir privilegios de administrador
  - **Formato**: ID de cuenta de sustrato de 32 bytes
  - **Requisitos**: Debe ser una cuenta válida (verificación de suma de comprobación).

**Devoluciones**:
- `Ok(())` - El administrador cambió exitosamente
- `Err(Error::Unauthorized)` - La persona que llama no es el administrador actual.

**Eventos emitidos**:
- [`AdminChanged`](crate::AdminChanged) - Registros de administradores y personas que llaman (antiguos/nuevos)

**Requisitos de seguridad**:
- **Control de acceso**: Solo el administrador actual puede llamar
- **Multi-sig recomendado**: Utilice la gobernanza para los cambios de producción.
- **Bloqueo de tiempo**: Considere el retraso por seguridad

**Costo de gasolina**: ~50,000 gas  
**Ejemplo**:
```rust
// Transferir administrador a una nueva billetera multifirma
contract.change_admin(new_multisig_wallet)?;
println!("Admin transferred successfully");
```

---

##### `set_oracle(oracle: AccountId) -> Result<(), Error>`

Configura la dirección del contrato del oráculo de precios.

**Parámetros**:
- `oracle` (`AccountId`) - Dirección del contrato de Oracle
  - **Requisitos**: Debe implementarse un contrato de Oracle

**Devoluciones**:
- `Ok(())` - Oracle configurado exitosamente
- `Err(Error::Unauthorized)` - La persona que llama no es administrador.

**Costo de gasolina**: ~30,000 gas  
**Ejemplo**:
```rust
// Configurar Oracle después de la implementación.
contract.set_oracle(oracle_contract_address)?;
```

---

##### `set_fee_manager(fee_manager: Option<AccountId>) -> Result<(), Error>`

Configura o elimina el contrato del gestor de tarifas.

**Parámetros**:
- `fee_manager` (`Option<AccountId>`) - Dirección del administrador de tarifas o `None` deshabilitar

**Devoluciones**:
- `Ok(())` - Configuración actualizada
- `Err(Error::Unauthorized)` - La persona que llama no es administrador.

**Costo de gasolina**: ~30,000 gas  

---

##### `set_compliance_registry(registry: Option<AccountId>) -> Result<(), Error>`

Configura o elimina el contrato del registro de cumplimiento.

**Parámetros**:
- `registry` (`Option<AccountId>`) - Dirección de registro de cumplimiento o `None`

**Devoluciones**:
- `Ok(())` - Configuración actualizada
- `Err(Error::Unauthorized)` - La persona que llama no es administrador.

**Costo de gasolina**: ~30,000 gas  

---

##### `update_valuation_from_oracle(property_id: u64) -> Result<(), Error>`

Actualiza la valoración de las propiedades utilizando la fuente de precios de Oracle.

**Parámetros**:
- `property_id` (`u64`) - ID de la propiedad a actualizar
  - **Restricciones**: Debe existir en el registro.

**Devoluciones**:
- `Ok(())` - Valoración actualizada correctamente
- `Err(Error::PropertyNotFound)` - La propiedad no existe
- `Err(Error::OracleError)` - La llamada de Oracle falló
- `Err(Error::OracleError)` - Oracle no configurado

**Eventos emitidos**:
- Evento de actualización de metadatos de la propiedad (indirectamente)

**Costo de gasolina**: ~75,000 gas (llamada entre contratos)
**Ejemplo**:
```rust
// Actualizar valoración antes de la venta.
contract.update_valuation_from_oracle(property_id)?;
let valuation = get_current_valuation(property_id);
```

---

##### `pause_contract(reason: String, duration_seconds: Option<u64>) -> Result<(), Error>`

Interrumpe todas las operaciones contractuales no críticas.

**Parámetros**:
- `reason` (`String`) - Motivo de la pausa legible para humanos
  - **Longitud máxima**: 1024 personajes
  - **Ejemplo**: `"Emergency maintenance - security audit"`
- `duration_seconds` (`Option<u64>`) - Retardo de reanudación automática opcional
  - **Ejemplo**: `Some(86400)` durante 24 horas
  - **Ninguno**: Se requiere currículum manual

**Devoluciones**:
- `Ok(())` - Contrato pausado exitosamente
- `Err(Error::NotAuthorizedToPause)` - La persona que llama no tiene permiso
- `Err(Error::AlreadyPaused)` - Contrato ya en pausa

**Eventos emitidos**:
- [`ContractPaused`](crate::ContractPaused) - Incluye motivo y tiempo de reanudación automática.

**Requisitos de seguridad**:
- **Control de acceso**: Solo administradores o guardianes de pausa
- **Usar con moderación**: Sólo situaciones de emergencia
- **Comunicación**: Anunciar la pausa públicamente

**Costo de gasolina**: ~50,000 gas  
**Ejemplo**:
```rust
// Pausa de emergencia
contract.pause_contract(
    "Critical vulnerability discovered".to_string(),
    None // Se requiere currículum manual
)?;
```

---

##### `emergency_pause(reason: String) -> Result<(), Error>`

Pausa inmediata sin reanudación automática (emergencias críticas).

**Parámetros**:
- `reason` (`String`) - Motivo de emergencia

**Devoluciones**: Igual que `pause_contract`  
**Costo de gasolina**: ~50,000 gas  
**Note**: equivalente a `pause_contract(reason, None)`

---

##### `try_auto_resume() -> Result<(), Error>`

Intenta reanudar el contrato si ha transcurrido el tiempo de reanudación automática.

**Parámetros**: Ninguno  
**Devoluciones**:
- `Ok(())` - Contrato reanudado con éxito
- `Err(Error::NotPaused)` - Contrato no pausado
- `Err(Error::ResumeRequestNotFound)` - No hay solicitud de currículum activa

**Eventos emitidos**:
- [`ContractResumed`](crate::ContractResumed)

**Costo de gasolina**: ~30,000 gas  

---

## Guía de manejo de errores

### Patrones de errores comunes

#### 1. Fallos de autorización

```rust
match contract.operation() {
    Ok(result) => process(result),
    Err(Error::Unauthorized) => {
        eprintln!("Access denied - check permissions");
        // Guíe al usuario para solicitar acceso.
    }
    Err(e) => handle_other_error(e),
}
```

#### 2. Fallos de cumplimiento

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

#### 3. Fallos de validación

```rust
// Validar previamente antes de enviar
fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), &'static str> {
    if metadata.location.is_empty() {
        return Err("Location required");
    }
    if metadata.valuation < 1000 {
        return Err("Minimum valuation $10");
    }
    Ok(())
}

// Entonces envía
match validate_metadata(&metadata) {
    Ok(_) => contract.register_property(metadata)?,
    Err(e) => eprintln!("Invalid metadata: {}", e),
}
```

### Referencia completa de errores

Ver [API_ERROR_CODES.md](./API_ERROR_CODES.md) para una documentación completa de todos los tipos de errores, incluidos:
- Condiciones de activación
- Escenarios comunes
- Pasos de recuperación
- Ejemplos
- equivalentes HTTP

---

## Ejemplos de integración

### Integración de frontend (React/TypeScript)

```typescript
import { useContract } from '@polkadot/react-hooks';

function RegisterPropertyForm() {
  const contract = useContract(CONTRACT_ADDRESS);
  
  const handleSubmit = async (metadata: PropertyMetadata) => {
    try {
      // Primero verifique el cumplimiento
      const isCompliant = await contract.query.checkAccountCompliance(
        currentUser.address
      );
      
      if (!isCompliant) {
        throw new Error('Complete KYC first');
      }
      
      // Enviar registro
      const tx = await contract.tx.registerProperty(metadata);
      await tx.signAndSend(currentUser.pair, ({ status, events }) => {
        if (status.isInBlock) {
          console.log('Transaction included in block');
          
          // Extraer el ID de la propiedad de los eventos
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
      {/* Campos de formulario */}
    </form>
  );
}
```

### Integración de backend (Node.js)

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function registerProperty(metadata) {
  const api = await ApiPromise.create({
    provider: new WsProvider('wss://rpc.propchain.io')
  });
  
  // Consultar el estado actual
  const health = await api.query.propertyRegistry.healthCheck();
  if (!health.isHealthy) {
    throw new Error('Contract not healthy');
  }
  
  // Comprobar cumplimiento
  const isCompliant = await api.query.complianceRegistry.isCompliant(
    userAddress
  );
  if (!isCompliant) {
    throw new Error('User not compliant');
  }
  
  // Enviar transacción
  const tx = api.tx.propertyRegistry.registerProperty(metadata);
  const hash = await tx.signAndSend(keypair);
  
  console.log('Transaction submitted:', hash.toHex());
  return hash;
}
```

### Integración de contratos inteligentes

```rust
// Patrón de llamada entre contratos
use ink::env::call::FromAccountId;

fn integrate_with_property_registry(
    registry_addr: AccountId,
    metadata: PropertyMetadata
) -> Result<u64, Error> {
    let registry: ink::contract_ref!(PropertyRegistry) = 
        FromAccountId::from_account_id(registry_addr);
    
    // Método de registro de llamadas
    let property_id = registry.register_property(metadata)?;
    
    Ok(property_id)
}
```

---

## Referencia de eventos

### Eventos clave a monitorear

#### `PropertyRegistered`

Se emite cuando se registra una nueva propiedad.

**Campos indexados** (filtrable):
- `property_id: u64`
- `owner: AccountId`

**Campos de datos**:
- `location: String`
- `size: u64`
- `valuation: u128`
- `timestamp: u64`
- `block_number: u32`
- `transaction_hash: Hash`

**Casos de uso**:
- Propiedad del índice
- Activar flujos de trabajo fuera de la cadena
- Actualizar paneles de análisis

---

#### `PropertyTransferred`

Se emite cuando cambia la propiedad de un inmueble.

**Campos indexados**:
- `property_id: u64`
- `from: AccountId`
- `to: AccountId`

**Casos de uso**:
- Actualizar registros de propiedad
- Calcular impuestos de transferencia
- Seguimiento de carteras de inversión

---

#### `EscrowCreated` / `EscrowReleased`

Realice un seguimiento del ciclo de vida del depósito en garantía para transferencias seguras.

**Casos de uso**:
- Monitorear el progreso de la transacción
- Detectar depósitos en garantía bloqueados
- Calcular las comisiones de depósito en garantía

---

## Consejos para la optimización del gas

### 1. Operaciones por lotes

```rust
// ❌ Costoso: Múltiples transacciones
for property in properties {
    contract.register_property(property)?;
}

// ✅ Más económico: transacción de un solo lote
contract.batch_register_properties(properties)?;
```

### 2. Prevalidación

```rust
// Valida primero fuera de la cadena para evitar desperdiciar gas.
if !validate_metadata_locally(&metadata) {
    return Err("Invalid metadata"); // Ahorra gasolina al no enviar
}
```

### 3. Consultas eficientes

```rust
// ❌ Costoso: Consulta en bucle
for id in property_ids {
    let prop = contract.get_property(id)?; // Multiple calls
}

// ✅ Mejor: Consulta por lotes si está disponible
let props = contract.get_properties_batch(property_ids)?; // llamada única
```

---

## Guía de prueba

### Pruebas unitarias

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
        
        // Marcar la llamada como cuenta no autorizada
        set_caller(unauthorized_account);
        
        let result = contract.change_admin(AccountId::from([2u8; 32]));
        assert!(matches!(result, Err(Error::Unauthorized)));
    }
}
```

### Pruebas de integración

```rust
#[ink_e2e::test]
async fn test_full_property_lifecycle(mut client: ink_e2e::Client<C, E>) {
    // Configuración
    let mut builder = build_contract!("propchain_contracts", "PropertyRegistry");
    let contract_id = client.instantiate("propchain_contracts", &bob, 0, &mut builder).await?;
    
    // Registrar propiedad
    let metadata = create_metadata();
    let register_msg = propchain_contracts::Message::RegisterProperty { metadata };
    let result = client.call(&bob, register_msg, &mut storage()).await?;
    
    // Verificar
    assert!(result.return_value().is_ok());
}
```

---

## Documentación relacionada

- **[API Documentation Standards](./API_DOCUMENTATION_STANDARDS.md)** - Cómo documentamos las API
- **[API Error Codes](./API_ERROR_CODES.md)** - Referencia completa de errores
- **[Architecture Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)** - Contexto del sistema
- **[Integration Guide](./integration.md)** - Patrones generales de integración
- **[Troubleshooting FAQ](./troubleshooting-faq.md)** - Problemas comunes

---

## Obtener Ayuda

### Recursos

- **GitHub Issues**: Para reportar errores o solicitar funciones (Tiempo de respuesta: 24-48 horas).
- **Discord**: Soporte para desarrolladores en tiempo real (Tiempo de respuesta: < 1 hora).
- **Stack Overflow**: Preguntas y respuestas técnicas (etiqueta: `propchain`).
- **Documentation**: Documentación completa en docs.propchain.io

### Canales de soporte

| Tipo de problema | Mejor canal | Tiempo de respuesta |
|------------|--------------|---------------|
| Informes de errores | Problemas de GitHub | 24-48 horas |
| Ayuda de integración | Discordia #dev-support | < 1 hora |
| Problemas de seguridad | security@propchain.io | Inmediato |
| Preguntas generales | Desbordamiento de pila | 2-24 horas |

---

**Última actualización**: April 22, 2026  
**Versión**: 1.0.0  
**Mantenido por**: Equipo de desarrollo de PropChain
