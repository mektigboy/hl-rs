# Hyperliquid Rust SDK

> **Beta Software - Not Production Ready**
>
> This SDK is under active development and should be considered beta software.
> APIs may change without notice. 
> **Always test thoroughly on testnet before using with real funds.** 

SDK inspired by the official [Hyperliquid Rust SDK](https://github.com/hyperliquid-dex/hyperliquid-rust-sdk) but with some changes that make it more flexible and suitable for advanced use cases.

## Usage Examples

### Quick Start - Send an Action

The simplest way to use the SDK is with `ExchangeClient::send_action()`:

```rust
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, UsdSend};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), hl_rs::Error> {
    // Create client with signer
    let wallet: PrivateKeySigner = "0x...".parse().unwrap();
    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);

    // Create and send an action in one call
    let action = UsdSend::new(Address::ZERO, dec!(1.0));
    let response = client.send_action(action).await?;
    
    println!("Response: {:?}", response);
    Ok(())
}
```

### Separate Prepare, Sign, Send Steps

For advanced use cases (external signing, batching, delayed sending):

```rust
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, UsdSend};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), hl_rs::Error> {
    let wallet: PrivateKeySigner = "0x...".parse().unwrap();
    let client = ExchangeClient::new(BaseUrl::Testnet);

    let action = UsdSend::new(Address::ZERO, dec!(1.0));

    // Step 1: Prepare (adds nonce, chain info, etc.)
    let prepared = client.prepare_action(action)?;

    // Step 2: Sign (can be done externally, e.g., AWS Nitro Enclaves)
    let signed = prepared.sign(&wallet)?;

    // Step 3: Send (when ready)
    let response = client.send_signed_action(signed).await?;
    
    Ok(())
}
```

### L1 Actions (Perp Deploy, Trading, etc.)

```rust
use alloy::primitives::Address;
use hl_rs::{
    ExchangeClient, BaseUrl,
    SetSubDeployers, SubDeployer, SubDeployerVariant,
    ToggleTrading, SetOpenInterestCaps,
};

// Grant oracle update permission to an address
let action = SetSubDeployers::new("mydex")
    .enable_permissions(
        Address::repeat_byte(0x42),
        vec![SubDeployerVariant::SetOracle],
    );

// Halt trading for a coin
let action = ToggleTrading::halt("mydex", "BTC");

// Set open interest caps
let action = SetOpenInterestCaps::new("mydex", vec![
    ("BTC", 10_000_000),
    ("ETH", 5_000_000),
]);
```

### User Signed Actions (Transfers, Withdrawals)

User signed actions use EIP-712 typed data and have builder patterns:

```rust
use alloy::primitives::Address;
use hl_rs::{UsdSend, Withdraw, SpotTransfer, ApproveAgent};
use rust_decimal_macros::dec;

// USDC transfer
let action = UsdSend::new(destination, dec!(100.0));

// Withdraw with builder
let action = Withdraw::builder()
    .destination(Address::ZERO)
    .amount(dec!(50.0))
    .build()
    .unwrap();

// Spot token transfer
let action = SpotTransfer::builder()
    .destination(Address::ZERO)
    .token("PURR")
    .amount(dec!(1000.0))
    .build()
    .unwrap();

// Approve an agent
let action = ApproveAgent::builder()
    .agent_address(agent_address)
    .agent_name("my-bot")
    .build()
    .unwrap();
```

### Vault Operations

```rust
use hl_rs::{ExchangeClient, BaseUrl};

let client = ExchangeClient::new(BaseUrl::Mainnet)
    .with_signer(wallet)
    .with_vault_address(vault_address);  // Actions will be on behalf of vault
```

## Resources

- [Hyperliquid API Reference](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api)
- [Hyperliquid Rust SDK](https://github.com/hyperliquid-dex/hyperliquid-rust-sdk)

---

## Implementing New Actions

This SDK uses derive macros to implement new actions with minimal boilerplate. There are two main action types, each with their own derive macro:

- **L1 Actions** - Signed using a phantom agent hash (msgpack-based)
- **User Signed Actions** - Signed using EIP-712 typed data

### L1 Actions (`#[derive(L1Action)]`)

L1 actions are the most common action type. They use msgpack serialization and a phantom agent signing scheme.

#### Flattened L1 Action (Simple)

When `action_type` and `payload_key` are the same (or `payload_key` is omitted), the action fields are flattened into the top-level payload.

```rust
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// A simple action where fields are flattened into the payload.
/// Serializes to: {"type": "updateLeverage", "asset": 0, "isCross": true, "leverage": 10}
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLeverage {
    pub asset: u32,
    pub is_cross: bool,
    pub leverage: u32,
    #[serde(skip_serializing)]  // nonce is not included in the payload
    pub nonce: Option<u64>,     // Required field for all L1 actions
}
```

**Key points:**
- The `action_type` defaults to the struct name in `lowerCamelCase` (e.g., `UpdateLeverage` → `"updateLeverage"`)
- When `action_type == payload_key`, struct fields are flattened into the root object
- The `nonce` field is **required** but should have `#[serde(skip_serializing)]`

#### Flattened L1 Action with Custom Type

You can override the `action_type` explicitly:

```rust
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// An action with a custom action type.
/// Serializes to: {"type": "noop"}
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "noop")]
pub struct NoOp {
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}
```

#### Nested L1 Action (Sum Type Pattern)

When `action_type` differs from `payload_key`, the action is nested under the `payload_key`. This is used for "sum type" actions like `perpDeploy` or `spotDeploy` which have multiple sub-actions.

```rust
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// A nested action for perp deployment operations.
/// Serializes to: {"type": "perpDeploy", "haltTrading": {"coin": "mydex:BTC", "isHalted": true}}
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "haltTrading")]
#[serde(rename_all = "camelCase")]
pub struct ToggleTrading {
    pub coin: String,
    pub is_halted: bool,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}
```

**Key points:**
- `action_type = "perpDeploy"` - The top-level type field value
- `payload_key = "haltTrading"` - The key under which action fields are nested
- This pattern groups related actions under a common type (e.g., all perp deploy actions share `"perpDeploy"`)

#### L1 Action with Array Payload

Some actions serialize their payload as an array rather than an object. Use the `flatten_vec!` macro:

```rust
use hl_rs_derive::L1Action;
use crate::flatten_vec;

/// An action where the payload is a sorted array.
/// Serializes to: {"type": "perpDeploy", "setOpenInterestCaps": [["mydex:BTC", 10000000]]}
#[derive(Debug, Clone, L1Action, Default)]
#[action(action_type = "perpDeploy", payload_key = "setOpenInterestCaps")]
pub struct SetOpenInterestCaps {
    pub caps: Vec<(String, u64)>,
    pub nonce: Option<u64>,
}

// This macro implements custom Serialize/Deserialize that:
// 1. Serializes only the `caps` field (not the full struct)
// 2. Sorts the array before serialization
flatten_vec!(SetOpenInterestCaps, caps);
```

### User Signed Actions (`#[derive(UserSignedAction)]`)

User signed actions use EIP-712 typed data signing. They require an explicit `types` attribute that defines the EIP-712 type string.

```rust
use hl_rs_derive::UserSignedAction;
use serde::{Deserialize, Serialize};
use derive_builder::Builder;
use alloy::primitives::Address;
use rust_decimal::Decimal;

/// Transfer USDC between Hyperliquid accounts.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(types = "UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)")]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct UsdSend {
    pub destination: Address,
    pub amount: Decimal,
    #[builder(default)]
    pub nonce: Option<u64>,  // Maps to "time" in the EIP-712 type
}
```

**Key points:**
- The `types` attribute defines the EIP-712 struct type
- Field names in the type string should match struct fields (camelCase in type, snake_case in Rust)
- `hyperliquidChain` is automatically provided by the signing context
- `time` or `nonce` in the type string maps to the `nonce` field
- The macro generates regular EIP-712 signing only; multisig now uses the dedicated wrapper flow (`send_multisig_action` with caller-provided inner signatures)

#### EIP-712 Type String Format

The `types` attribute follows EIP-712 format:
```
StructName(type1 field1,type2 field2,...)
```

Common types:
- `string` - String values (addresses are often serialized as lowercase strings)
- `uint64` - Unsigned 64-bit integers
- `address` - Ethereum addresses
- `bool` - Boolean values

### Summary Table

| Pattern | Macro | Serialization | Use Case |
|---------|-------|---------------|----------|
| Flattened L1 | `L1Action` | `{"type": "X", ...fields}` | Simple actions |
| Nested L1 | `L1Action` | `{"type": "X", "key": {...fields}}` | Sum type actions (perpDeploy, spotDeploy) |
| Array L1 | `L1Action` + `flatten_vec!` | `{"type": "X", "key": [[...]]}` | List-based configs |
| User Signed | `UserSignedAction` | EIP-712 typed data | Transfers, withdrawals |

### Required Fields

All actions **must** have a `nonce: Option<u64>` field. This is enforced at compile time by the derive macros.

### Adding to ActionKind

After creating a new action, add it to the `ActionKind` enum in `src/actions/mod.rs` to make it usable through the standard action flow.
