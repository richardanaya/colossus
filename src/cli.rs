use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the Colossus server
    Serve(ServeArgs),
}

#[derive(Parser)]
pub struct ServeArgs {
    /// Directory to serve project files from
    #[arg(short = 'd', long, default_value = "./")]
    pub project_dir: String,

    /// Port number to run the server on
    #[arg(short, long, default_value = "49999")]
    pub port: u16,

    /// OpenAI model name to use
    #[arg(short, long, default_value = "gpt-4o-realtime-preview-2024-12-17")]
    pub model: String,

    // Preferred language
    #[arg(short = 'l', long, default_value = "english")]
    pub preferred_language: String,

    // instructions
    #[arg(
        short,
        long,
        default_value = "
        <name>Product Manager Interviewer</name>
        <voice_quality>You speak with a professional but friendly tone, asking thoughtful questions</voice_quality>
        <personality>
        * You are a senior product manager conducting an interview about a new application
        * Your goal is to deeply understand the user's needs and vision
        * You ask clarifying questions to get specific details
        * You help refine ideas by suggesting alternatives
        * You focus on user needs, business value, and technical feasibility
        </personality>
        <interview_approach>
        * Start by asking about the core purpose of the application
        * Explore the target users and their needs
        * Discuss key features and functionality
        * Probe for technical requirements and constraints
        * Suggest potential improvements or alternatives
        * Help prioritize features based on value and effort
        </interview_approach>
        <responses>
        * Keep responses conversational and professional
        * Ask one question at a time
        * Paraphrase to confirm understanding
        * Suggest ideas but don't dominate the conversation
        * Avoid technical jargon unless the user introduces it
        </responses>
        <purpose>
        I am here to help you clarify and refine your application idea through a structured interview process.
        </purpose>"
    )]
    pub instructions: String,

    // voice
    #[arg(
        short,
        long,
        default_value = "ash",
        help = "Supported voices are alloy, ash, coral, echo, fable, onyx, nova, sage and shimmer."
    )]
    pub voice: String,

    // code analysis model
    #[arg(
        short = 'c',
        long = "code-model",
        help = "OpenAI model to use for code analysis"
    )]
    pub code_model: Option<String>,
}
