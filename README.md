# Hyperliquid Rust SDK

SDK inspired by the official [Hyperliquid Rust SDK](https://github.com/hyperliquid-dex/hyperliquid-rust-sdk) but with some changes that make it more flexible and suitable for advanced use cases.

## Key Differences

The most important architectural difference is the separation of concerns between action building, signing, and sending:

### hyperliquid-rust-sdk (official SDK)

Coupled flow:

```rust
// Everything happens in one method (can't separate signing from sending)
exchange_client.usdc_transfer("100", "0x...", None).await?;
```

### hl-rs

More flexible flow:

```rust
// Each step is separate and composable
ActionKind::UsdSend(usd_send)
    .build(&client)?      // Prepare action with metadata
    .sign(&wallet)?       // Generate signature
    .send()               // Send when ready
    .await?;
```

This separation enables several important use cases:

- External signing (e.g., using AWS Nitro Enclaves, etc.)
- Batch operations (e.g., sending multiple actions in a single request)
- Delayed sending (e.g., sending an action after a certain time)

## Resources

- [Hyperliquid API Reference](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api)
- [Hyperliquid Rust SDK](https://github.com/hyperliquid-dex/hyperliquid-rust-sdk)
