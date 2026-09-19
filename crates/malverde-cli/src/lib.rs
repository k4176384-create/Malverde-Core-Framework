//! # Malverde CLI
//!
//! Command-line interface for the Malverde Framework.
//! Provides CLI commands and argument parsing.

pub mod commands;
pub mod error;

pub use commands::*;
pub use error::*;

/// CLI application
#[derive(Debug, Clone)]
pub struct CliApp {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl CliApp {
    pub fn new() -> Self {
        CliApp {
            name: "malverde".to_string(),
            version: "0.1.0".to_string(),
            description: "Malverde Core Framework CLI".to_string(),
        }
    }

    pub fn run(&self) -> Result<(), CliError> {
        use clap::{Parser, Subcommand};
        
        #[derive(Parser)]
        #[command(name = "malverde")]
        #[command(author = "Malverde Team")]
        #[command(version = "0.1.0")]
        #[command(about = "Malverde Core Framework CLI")]
        struct Cli {
            #[command(subcommand)]
            command: Commands,
        }

        #[derive(Subcommand)]
        enum Commands {
            /// Database operations
            Db(DbCommands),
            /// Knowledge operations
            Knowledge(KnowledgeCommands),
            /// Memory operations
            Memory(MemoryCommands),
            /// Job operations
            Job(JobCommands),
            /// Event operations
            Event(EventCommands),
            /// Audit operations
            Audit(AuditCommands),
            /// Configuration operations
            Config(ConfigCommands),
            /// System operations
            System(SystemCommands),
        }

        let cli = Cli::parse();
        
        match cli.command {
            Commands::Db(cmd) => self.handle_db_command(cmd),
            Commands::Knowledge(cmd) => self.handle_knowledge_command(cmd),
            Commands::Memory(cmd) => self.handle_memory_command(cmd),
            Commands::Job(cmd) => self.handle_job_command(cmd),
            Commands::Event(cmd) => self.handle_event_command(cmd),
            Commands::Audit(cmd) => self.handle_audit_command(cmd),
            Commands::Config(cmd) => self.handle_config_command(cmd),
            Commands::System(cmd) => self.handle_system_command(cmd),
        }
    }

    fn handle_db_command(&self, cmd: DbCommands) -> Result<(), CliError> {
        commands::handle_db_command(cmd)
    }

    fn handle_knowledge_command(&self, cmd: KnowledgeCommands) -> Result<(), CliError> {
        commands::handle_knowledge_command(cmd)
    }

    fn handle_memory_command(&self, cmd: MemoryCommands) -> Result<(), CliError> {
        commands::handle_memory_command(cmd)
    }

    fn handle_job_command(&self, cmd: JobCommands) -> Result<(), CliError> {
        commands::handle_job_command(cmd)
    }

    fn handle_event_command(&self, cmd: EventCommands) -> Result<(), CliError> {
        commands::handle_event_command(cmd)
    }

    fn handle_audit_command(&self, cmd: AuditCommands) -> Result<(), CliError> {
        commands::handle_audit_command(cmd)
    }

    fn handle_config_command(&self, cmd: ConfigCommands) -> Result<(), CliError> {
        commands::handle_config_command(cmd)
    }

    fn handle_system_command(&self, cmd: SystemCommands) -> Result<(), CliError> {
        commands::handle_system_command(cmd)
    }
}

impl Default for CliApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the CLI application
pub fn run() -> Result<(), CliError> {
    let app = CliApp::new();
    app.run()
}
