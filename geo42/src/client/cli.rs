use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "geo42-cli",
    about = "A command line interface for geo42 server",
    long_about = "geo42-cli is a command line client for the geo42 geospatial database server.\nIt allows you to connect to a geo42 server and execute commands interactively or non-interactively."
)]
pub struct CliArgs {
    /// Server hostname
    #[arg(long = "host", default_value = "127.0.0.1")]
    pub host: String,

    /// Server port
    #[arg(short = 'p', long = "port", default_value = "9851")]
    pub port: u16,

    /// Enter interactive mode
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Command to execute (if not in interactive mode)
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.interactive && self.command.is_empty() {
            return Err("No command specified. Use -i for interactive mode or provide a command.".to_string());
        }
        
        if self.port == 0 {
            return Err("Port must be greater than 0".to_string());
        }
        
        Ok(())
    }

    pub fn should_run_interactive(&self) -> bool {
        self.interactive || self.command.is_empty()
    }
}
