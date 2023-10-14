# Node metadata manage.
Every blockchain based on Substrate has its unique metadata. In this repository, we manage this metadata through rust features. For example, if we want to use ./metadata/substrate_metadata.scale, we need to use 
```
cargo build --release --features substrate.    
```
This will trigger the code:
```
#[cfg(feature = "substrate")]
#[subxt::subxt(runtime_metadata_path = "metadata/substrate_metadata.scale")]
pub mod substrate {}
```

There is a rust-analyzer.cargo.features setting where you can specify a list of features.

