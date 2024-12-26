<p align="center">
  <img src="pyramid.png" width="200" />
</p>

# Colossus

**Aider + OpenAI Advanced Voice Model = Perfect Coding Companion**

Fair warning: this is very hot of the presses. I'm still working on a ideal workflow and actions you can do and nice sounding prompts, but it does work!

Colossus is a powerful real-time voice assistant designed to streamline your development workflow by providing voice-controlled integration with [`aider`](https://aider.chat/), an AI pair programming tool.

Disclaimer: this project uses [realtime API pricing](https://openai.com/api/pricing/). Make sure that's compatible with your budget. **Please note that with your credits at tier 1 levels, you will be limited to a number of requests PER DAY. I had to spend $50 credits to no longer be capped on per day limitations.**

<img width="1513" alt="Screenshot 2024-12-25 at 11 53 49 PM" src="https://github.com/user-attachments/assets/802e4007-40fe-453b-aeab-cf8459c87464" />


## Features

- **Intelligent Code Analysis**: Ask questions about your codebase 
- **Code Modification**: Make changes to your codebase through voice commands
- **Context Switching**: Seamlessly switch between different parts of your project by using dynamic `/load` of context files based on the conversation
- **Web Search Integration**: Search the web using Perplexity to gather additional information and context

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

## Quick Start

TLDR: you should be able to run colossus anywhere you normally run aider as long as there is a .git repo and OPENAI_API_KEY

1. Ensure you have a `.env` file in your project directory with an OPENAI_API_KEY (same directory where you run aider) or its in your environment variables
2. Launch Colossus:
   ```bash
   colossus
   ```
3. Go to the website url listed
4. Click start sessions
5. Ask questions
6. Hit mute/unmute as needed

## Advanced Usage

Colossus supports several command line options for customization:

```terminal
USAGE:
    colossus [OPTIONS]

OPTIONS:
    -d, --project-dir <DIR>         Directory to serve project files from [default: "./"]
    -p, --port <PORT>               Port number to run the server on [default: 49999]
    -m, --model <MODEL>             OpenAI model name to use [default: gpt-4o-mini-realtime-preview-2024-12-17]
    -l, --preferred-language <LANG> Preferred language for communication [default: english]
    -i, --instructions <TEXT>       Custom instructions for the AI assistant
    -v, --voice <VOICE>            Voice to use for speech [default: ash]
    -h, --help                     Print help information
    -V, --version                  Print version information
```

Example with custom settings:
```bash
colossus --port 3000 --model gpt-4o-mini-realtime-preview-2024-12-17 --project-dir /path/to/project
```

## Context

You can have various aider context files that can be loaded in by using the aider `/load` command.

Any file that is prefixed in the root directory `CONTEXT_` and ends with extension `.md` will show up as a button you can load.

Example:

```markdown
// CONTEXT_webpage.md - a context that clears context and adds all relevant web page files
/drop
/add **.*.html
/add **.*.js
```


## Contributing

This is all incredibly new, but feel free to drop suggestions!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Art

![immortalrobot_man_talking_to_super_computer_AI_and_voice_orie_8ca16ca3-3eee-4ffd-9233-433652c7bca7_1](https://github.com/user-attachments/assets/19620597-531b-4c79-9802-adc8162f36b1)
