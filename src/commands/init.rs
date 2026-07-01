use crate::io;
use anyhow::Result;
use std::path::PathBuf;

pub fn init_config(force: bool) -> Result<()> {
    let config_path = PathBuf::from(".debtmap.toml");

    if config_path.exists() && !force {
        anyhow::bail!("Configuration file already exists. Use --force to overwrite.");
    }

    let default_config = r#"# Debtmap Configuration

[thresholds]
complexity = 10
duplication = 50
max_file_length = 500
max_function_length = 50

[languages]
enabled = ["rust", "python", "javascript", "typescript", "go", "solidity"]

[languages.rust]
detect_dead_code = false
detect_complexity = true
detect_duplication = true

[languages.python]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.javascript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.typescript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.go]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
generated_code = "suppress_debt"

[languages.solidity]
detect_dead_code = false
detect_complexity = true
detect_duplication = true
vendor_code = "suppress_debt"
large_contract_threshold = 20

[languages.solidity.security]
tx_origin = true
reentrancy_heuristic = true
unchecked_calls = true
delegatecall = true
selfdestruct = true
assembly_blocks = true
unbounded_loops = true
missing_access_control = true
hardcoded_addresses = true
floating_pragma = true
large_contracts = true
unchecked_arithmetic = true
unsafe_erc20_transfer = true
push_without_length_cap = true
block_timestamp_dependency = true
tx_gas_price_dependency = true
encode_packed_collision = true
delegatecall_in_constructor = true

[ignore]
patterns = [
    "target/**",
    "venv/**",
    "node_modules/**",
    "out/**",
    "cache/**",
    "artifacts/**",
    "*.min.js"
]

[output]
default_format = "terminal"
"#;

    io::write_file(&config_path, default_config)?;
    println!("Created .debtmap.toml configuration file");

    Ok(())
}
