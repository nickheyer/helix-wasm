use anyhow::Result;

// This binary is used in the Release CI as an optimization to cut down on
// compilation time. This is not meant to be run manually.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    helix_loader::grammar::fetch_grammars()
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> {
    anyhow::bail!("grammar fetching is not supported on wasm32")
}
