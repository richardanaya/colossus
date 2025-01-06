<p align="center">
  <img src="pyramid.png" width="200" />
</p>

# Colossus

**Voice-Controlled Multi-Agent AI Development System**

Colossus is an innovative development platform that combines real-time voice interaction with a coordinated team of AI agents to streamline the software development process. It orchestrates multiple specialized AI agents working together through different phases of development while maintaining voice communication with you.

## How It Works

Colossus operates in three distinct phases:

### 1. Planning Phase
During this phase, multiple agents work simultaneously:
- **Product Manager**: Processes voice transcripts to maintain project requirements
- **Architect**: Designs and updates the technical architecture
- **Project Manager**: Breaks down work into specific tasks
- **Test Strategist**: Develops comprehensive test plans

### 2. Development Phase
Once planning is complete, the system switches to development mode where:
- **Developer Agent**: Implements tasks in order, following test-driven development
- Automated build and test processes run after each implementation
- Tasks are automatically marked complete when tests pass

### 3. Human Intervention Mode
If critical issues arise that AI cannot resolve:
- System automatically halts development
- Signals need for human intervention
- Provides detailed error context
- Returns to development mode once issues are resolved

![Untitled drawing (5)](https://github.com/user-attachments/assets/f9f997bb-22ae-42be-b14f-f982810cc1b3)


## Key Features

- **Real-time Voice Interface**: Natural conversation with the AI system
- **Multi-Agent Coordination**: Specialized AI agents working in concert
- **Automated Development Cycle**: Continuous implementation, testing, and validation
- **Context-Aware Development**: Uses multiple context files for specialized tasks
- **Web Search Integration**: Perplexity-powered web search for additional information
- **Error Management**: Smart detection and handling of critical issues

## Important Notes

This project uses OpenAI's real-time API pricing. Please be aware:
- Requires appropriate API credits and budget
- Tier 1 credits have daily request limits
- Recommended minimum credit balance: $50 for unrestricted usage

<img width="700" alt="Screenshot 2025-01-05 at 7 15 00 PM" src="https://github.com/user-attachments/assets/61ced4be-b020-421f-ab10-fa5b4ae9cb5d" />


## Installation

1. Install Rust and Cargo using rustup:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Restart your terminal or reload your shell configuration:
   ```bash
   source "$HOME/.cargo/env"
   ```

3. Install Colossus:
   ```bash
   cargo install colossus
   ```

## Required API Keys

To use all features of Colossus, you'll need the following API keys:

- **OpenAI API Key**: Required for real-time voice interactions and code analysis (based off [aider leader board](https://aider.chat/docs/leaderboards/) )
  - Get it from: https://platform.openai.com/api-keys
  - Set as: `OPENAI_API_KEY`

- **Perplexity API Key**: Required for web search integration
  - Get it from: https://www.perplexity.ai/settings/api
  - Set as: `PERPLEXITY_API_KEY`

You can set these keys in your `.env` file or as environment variables.

## Quick Start

1. Create a new project:
   ```bash
   colossus init ./my-project
   ```
   This will:
   - Create the project directory if it doesn't exist
   - Initialize a git repository
   - Create a template .env file
   - Set up language-specific configuration files
   - Create a Makefile with build and test targets

2. Add your API keys to the .env file:
   - OPENAI_API_KEY (required)
   - PERPLEXITY_API_KEY (optional, for web search)

3. Start the Colossus server:
   ```bash
   colossus serve
   ```

4. Open the web interface shown in the terminal
5. Click "Start Session" to begin
6. Use the microphone button to talk with Colossus

## Advanced Usage

Colossus supports several command line options for customization:

```terminal
USAGE:
    colossus [OPTIONS]

OPTIONS:
    -d, --project-dir <DIR>         Directory to serve project files from [default: "./"]
    -p, --port <PORT>               Port number to run the server on [default: 49999]
    -m, --model <MODEL>             OpenAI model name to use [default: gpt-4o-realtime-preview-2024-12-17]
    -l, --preferred-language <LANG> Preferred language for communication [default: english]
    -i, --instructions <TEXT>       Custom instructions for the AI assistant
    -v, --voice <VOICE>             Voice to use for speech [default: ash] (supported: alloy, ash, coral, echo, fable, onyx, nova, sage, shimmer)
    -c, --code-model <MODEL>        OpenAI model to use for code analysis
    -h, --help                      Print help information
    -V, --version                   Print version information
```

Example with custom settings:
```bash
colossus -c deepseek/deepseek-chat -d /path/to/project
```

## How to prepare a project for colossus

* add a `Makefile` that has a `build` and `test` target
* add  a `CONTEXT.md` that loads all appropriate files
  
```
# example
/add TASKS.md
/read-only ARCHITECTURE.md
/read-only PROJECT.md
/read-only TEST_STRATEGY.md
/read-only Makefile
/add **/\*.js
/add **/_.css
/add \*\*/_.html
/add package.json
```


## Contributing

This is all incredibly new, but feel free to drop suggestions!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Art

![immortalrobot_man_talking_to_super_computer_AI_and_voice_orie_8ca16ca3-3eee-4ffd-9233-433652c7bca7_1](https://github.com/user-attachments/assets/19620597-531b-4c79-9802-adc8162f36b1)
