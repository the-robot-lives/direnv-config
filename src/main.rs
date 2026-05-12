mod cmd;
mod store;
mod yaml;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dc", version, about = "YAML-backed configuration layer for direnv")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Merge YAML from stdin into a named config
    Yaml {
        name: String,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        replace: bool,
        #[arg(long, value_name = "KEY")]
        replace_key: Option<String>,
        #[arg(long)]
        if_missing: bool,
        #[arg(long)]
        no_bump: bool,
    },
    /// Read a config value by path
    Get {
        name: String,
        path: Option<String>,
        #[arg(long)]
        raw: bool,
    },
    /// Set a config value
    Set {
        name: String,
        key: String,
        value: String,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Remove a key from a named config
    Unset {
        name: String,
        keys: Vec<String>,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Remove named configs or branches within them
    Prune {
        name: String,
        keys: Vec<String>,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Export resolved config as shell env vars
    Env {
        #[arg(long)]
        list: bool,
        #[arg(long)]
        diff: bool,
    },
    /// Bump the version counter
    Bump,
    /// Initialize a config store for the current directory
    Init {
        #[arg(long)]
        from_envrc: Option<String>,
    },
    /// Show current config state
    Status,
    /// List all known config stores
    List,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Yaml { name, layer, replace, replace_key, if_missing, no_bump } => {
            cmd::yaml::run(&name, layer.as_deref(), replace, replace_key.as_deref(), if_missing, no_bump)
        }
        Commands::Get { name, path, raw } => {
            cmd::get::run(&name, path.as_deref(), raw)
        }
        Commands::Set { name, key, value, layer, no_bump } => {
            cmd::set::run(&name, &key, &value, layer.as_deref(), no_bump)
        }
        Commands::Unset { name, keys, layer, no_bump } => {
            cmd::unset::run(&name, &keys, layer.as_deref(), no_bump)
        }
        Commands::Prune { name, keys, layer, no_bump } => {
            cmd::prune::run(&name, &keys, layer.as_deref(), no_bump)
        }
        Commands::Env { list, diff } => {
            cmd::env::run(list, diff)
        }
        Commands::Bump => {
            cmd::bump::run()
        }
        Commands::Init { from_envrc } => {
            cmd::init::run(from_envrc.as_deref())
        }
        Commands::Status => {
            cmd::status::run()
        }
        Commands::List => {
            cmd::list::run()
        }
    }
}
