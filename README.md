<p align="center">
  <img src="pyramid.png" width="200" />
</p>

# Colossus

Colossus is a powerful real-time voice assistant designed to streamline your development workflow by providing voice-controlled integration with [`aider`](https://aider.chat/), an AI pair programming tool.

## Features

- **Intelligent Code Analysis**: Ask questions about your codebase and receive immediate responses
- **Code Modification**: Make changes to your codebase through voice commands
- **Context Switching**: Seamlessly switch between different parts of your project

## Installation

```bash
cargo install colossus
```

## Quick Start

1. Ensure you have a `.env` file in your project directory with an OPENAI_API_KEY (same directory where you run aider)
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

```bash
USAGE:
    colossus [OPTIONS]

OPTIONS:
    -p, --port <PORT>       Port number to run the server on [default: 49999]
    -m, --model <MODEL>     OpenAI model name to use [default: gpt-4o-mini-realtime-preview-2024-12-17]
    -d, --project-dir <DIR> Directory AI will operate aider on [default: "./"]
    -h, --help             Print help information
    -V, --version          Print version information
```

Example with custom settings:
```bash
colossus --port 3000 --model gpt-4o-mini-realtime-preview-2024-12-17 --project-dir /path/to/project
```
## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Art

![immortalrobot_man_talking_to_super_computer_AI_and_voice_orie_8ca16ca3-3eee-4ffd-9233-433652c7bca7_1](https://github.com/user-attachments/assets/19620597-531b-4c79-9802-adc8162f36b1)
