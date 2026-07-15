# 🧠 OpenTUI

> **A stunning terminal AI chat interface** — A lightweight, blazing-fast alternative to Open Web UI that runs entirely in your terminal.

![License: MIT](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)

---

## ✨ Features

- **🤖 Multi-Provider Support** — OpenAI, Groq, NVIDIA NIM, Anthropic, Google Gemini, DeepSeek, Mistral AI
- **⚡ Streaming Responses** — Real-time AI responses as they're generated
- **🎨 Beautiful TUI** — Ratatui-powered interface with gorgeous Catppuccin-inspired colors
- **🌐 Web Settings UI** — Configure everything from your browser (`Ctrl+S`)
- **🎭 Multiple Themes** — Catppuccin, Classic Terminal, Nord, Dracula, Tokyo Night, Gruvbox
- **🔧 Developer Mode** — Monitor tokens/sec, costs, response times and more
- **📦 Auto-Discover Models** — Fetch available models directly from provider APIs
- **💾 Persistent Config** — Settings saved automatically to `~/.config/opentui/config.toml`
- **⌨️ Keyboard-Centric** — Full keyboard navigation with intuitive shortcuts

---

## 🚀 Installation

### Quick Install (cargo)

```bash
cargo install opentui
```

### From Source

```bash
# Clone the repository
git clone https://github.com/rashmina-a/opentui.git
cd opentui

# Build and run
cargo run --release

# Or install globally
cargo install --path .
```

### One-liner (development)

```bash
git clone https://github.com/rashmina-a/opentui.git && cd opentui && cargo run
```

### Requirements

- **Rust** 1.75 or later ([install via rustup](https://rustup.rs/))
- A terminal emulator with good Unicode support (Kitty, Alacritty, iTerm2, Windows Terminal, etc.)

---

## 🎮 Usage

### Quick Start

1. **Launch OpenTUI:**
   ```bash
   opentui
   ```
2. **Configure your API key:**
   - Press `Ctrl+S` to open settings in your browser
   - Or use the TUI settings panel (press `Esc`, then navigate)
3. **Start chatting!**
   - Type your message and press `Enter`

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Open web settings interface |
| `Ctrl+Q` / `Ctrl+C` | Quit |
| `Ctrl+L` | Clear conversation |
| `Enter` | Send message / Confirm |
| `Esc` | Cancel streaming / Close settings |
| `Tab` | Next field (in settings) |
| `↑/↓` | Navigate / Scroll |
| `←/→` | Switch tabs (in settings) |
| `Page Up/Down` | Scroll through messages |

### CLI Options

```bash
opentui [OPTIONS]

Options:
  -p, --provider <PROVIDER>  Provider to use (openai, groq, nvidia, anthropic, google, deepseek, mistral)
  -m, --model <MODEL>        Model to use
  -d, --dev                  Enable developer mode
      --config-path          Show config file path and exit
      --message <MESSAGE>    Send a message and exit (non-interactive)
  -h, --help                 Print help
  -V, --version              Print version
```

**Examples:**
```bash
# Launch with a specific provider
opentui --provider groq

# Send a one-shot message
opentui --message "What is Rust?" --provider openai

# Enable developer mode
opentui --dev

# Show config path
opentui --config-path
```

---

## 🎨 Themes

OpenTUI comes with **6 beautiful themes** that you can switch between:

| Theme | Description |
|-------|-------------|
| **Catppuccin Mocha** | Smooth purple/pink/teal palette (default) |
| **Classic Terminal** | Green-on-black retro terminal aesthetic |
| **Nord** | Arctic blue/cyan tones |
| **Dracula** | Rich purple/green contrast |
| **Tokyo Night** | Deep blue/purple night theme |
| **Gruvbox** | Warm earthy yellow/green tones |

Change themes in **General Settings** (`Ctrl+S` → General tab → Theme).

---

## 🌐 Web Settings Interface

Open the web settings panel with `Ctrl+S` to configure everything in your browser:

- **Providers Tab** — Set API keys, base URLs, temperature, and model selection
- **General Tab** — Default provider, scrollback lines, and theme selection with live preview
- **Developer Tab** — Toggle developer mode and performance metrics
- **Auto Model Discovery** — Enter a custom base URL and OpenTUI will fetch available models automatically

---

## 🔧 Configuration

Settings are stored in `~/.config/opentui/config.toml` and can be edited directly:

```toml
[ui]
theme = "catppuccin"
developer_mode = false
scrollback_lines = 1000

[openai]
api_key = "sk-..."
model = "gpt-4o"
temperature = 0.7
max_tokens = 4096
```

### Environment Variables

You can also set API keys via environment variables:
- `OPENAI_API_KEY`
- `GROQ_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`
- `DEEPSEEK_API_KEY`
- `MISTRAL_API_KEY`
- `NVIDIA_API_KEY`

---

## 🏗️ Project Structure

```
src/
├── main.rs              # Entry point, CLI, event loop, web server
├── app.rs               # Application state and logic
├── config.rs            # Configuration management
├── chat.rs              # Chat and conversation state
├── dev_mode.rs          # Developer mode metrics
├── providers/           # AI provider implementations
│   ├── mod.rs           # Provider trait and factory
│   ├── openai.rs        # OpenAI-compatible API
│   ├── groq.rs          # Groq API
│   ├── nvidia.rs        # NVIDIA NIM API
│   ├── anthropic.rs     # Anthropic API
│   ├── google.rs        # Google Gemini API
│   ├── deepseek.rs      # DeepSeek API
│   └── mistral.rs       # Mistral AI API
└── ui/
    ├── mod.rs           # Color theme, screen enum, helpers
    ├── chat_screen.rs   # Chat UI rendering
    └── settings_screen.rs # Settings overlay UI
```

---

## 🤝 Contributing

Contributions are welcome! The project is still in active development. Feel free to:

- Report bugs and suggest features via [Issues](https://github.com/rashmina-a/opentui/issues)
- Submit pull requests for improvements
- Add support for new AI providers

---

## 🙏 Credits

- **Original idea** by [Vihas Methnula](https://github.com/VihasMethnula)
- Built with [Ratatui](https://ratatui.rs/) and [Warp](https://github.com/seanmonstar/warp)

---

## 📄 License

MIT
