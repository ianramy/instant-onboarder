# 🚀 Instant Onboarder

**Slashing developer onboarding from weeks to minutes using privacy-first local file traversal and JIT (Just-In-Time) AI intelligence.**

Instant Onboarder is a cutting-edge CLI tool that revolutionizes how developers understand new codebases. By combining intelligent file scanning, SHA-256 caching, and dual AI backend support (IBM watsonx.ai + local Ollama), it delivers instant, context-aware code explanations through a beautiful terminal UI.

---

## 🏆 Why It Wins

### 1. 🔒 Zero-Friction Privacy (Local-First)
- **No cloud uploads required** - All file scanning happens locally
- **You control your data** - Choose between cloud AI (watsonx.ai) or fully local AI (Ollama)
- **Smart filtering** - Automatically ignores `.git`, `node_modules`, `target`, and other build artifacts
- **50+ language support** - From Rust to Python, JavaScript to Go

### 2. ⚡ Token Optimized (Aggressive SHA-256 Caching)
- **Zero redundant API calls** - SHA-256 hashing ensures identical files are never processed twice
- **Persistent cache** - Explanations survive across sessions
- **Instant retrieval** - Cached explanations load in milliseconds
- **Cost-effective** - Dramatically reduces API token consumption

### 3. 🎨 Premium DX (Ratatui TUI)
- **Split-pane interface** - File explorer on the left, deep dive on the right
- **Keyboard-driven** - Navigate with arrows, analyze with Enter, quit with 'q'
- **Real-time feedback** - Loading states, error handling, and progress indicators
- **Beautiful errors** - Powered by `miette` for diagnostic-quality error messages
- **Graceful cleanup** - Terminal state always restored, even on crashes

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **AI Backend** (choose one):
  - **IBM watsonx.ai** - Cloud-based, requires API key
  - **Ollama** - Local, requires [Ollama](https://ollama.ai) installed with `granite-code` model

### Installation

```bash
# Clone the repository
git clone https://github.com/ianramy/instant-onboarder.git
cd instant-onboarder

# Build the release binary
cargo build --release

# The binary will be at: ./target/release/instant-onboarder
```

### First-Time Setup

```bash
# Run interactive setup to configure your AI backend
./target/release/instant-onboarder --setup
```

You'll be prompted to choose:
1. **watsonx.ai** - Enter your IBM Cloud API key
2. **Local Ollama** - Ensure Ollama is running with `granite-code` model

Configuration is saved to `~/.config/instant-onboarder/config.json`

---

## 📖 Usage

### Analyze a Codebase

```bash
# Analyze current directory
./target/release/instant-onboarder

# Analyze specific directory
./target/release/instant-onboarder /path/to/project

# Analyze with verbose output
./target/release/instant-onboarder ./my-project
```

### TUI Controls

Once the TUI launches:
- **↑/↓** - Navigate through files
- **Enter** - Analyze selected file (checks cache first, then calls AI)
- **q** or **Esc** - Quit application

### Cache Management

```bash
# Clear all cached explanations
./target/release/instant-onboarder --clear-cache

# Re-run setup (reconfigure AI backend)
./target/release/instant-onboarder --setup
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Instant Onboarder                        │
├─────────────────────────────────────────────────────────────┤
│  CLI Layer (clap)                                            │
│    ├─ Argument parsing                                       │
│    ├─ Flag handling (--setup, --clear-cache)                │
│    └─ Target directory selection                            │
├─────────────────────────────────────────────────────────────┤
│  Config Layer (serde + directories)                          │
│    ├─ Interactive setup (dialoguer)                          │
│    ├─ JSON persistence                                       │
│    └─ Cross-platform paths                                  │
├─────────────────────────────────────────────────────────────┤
│  Parser Layer (walkdir)                                      │
│    ├─ Recursive directory traversal                          │
│    ├─ Smart filtering (ignore build dirs)                   │
│    └─ Extension-based file selection                        │
├─────────────────────────────────────────────────────────────┤
│  Engine Layer (reqwest + sha2)                               │
│    ├─ CacheManager (SHA-256 hashing)                         │
│    ├─ AiClient (dual backend support)                        │
│    │   ├─ Ollama integration (local)                         │
│    │   └─ watsonx.ai integration (cloud)                     │
│    └─ Async processing (tokio)                              │
├─────────────────────────────────────────────────────────────┤
│  UI Layer (ratatui + crossterm)                              │
│    ├─ Split-pane layout                                      │
│    ├─ Event-driven navigation                                │
│    ├─ Loading states                                         │
│    └─ Graceful terminal cleanup                             │
├─────────────────────────────────────────────────────────────┤
│  Error Layer (thiserror + miette)                            │
│    ├─ Typed error variants                                   │
│    ├─ Rich diagnostics                                       │
│    └─ Helpful error messages                                │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ Development

### Build from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Check for errors
cargo check

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```bash
instant-onboarder/
├── src/
│   ├── main.rs       # Entry point & orchestration
│   ├── cli.rs        # Command-line interface
│   ├── config.rs     # Configuration management
│   ├── errors.rs     # Error types & diagnostics
│   ├── parser.rs     # File scanning & filtering
│   ├── engine.rs     # AI client & caching
│   └── ui.rs         # Terminal UI (Ratatui)
├── Cargo.toml        # Dependencies
└── README.md         # This file
```

---

## 🎯 Key Features

- ✅ **Dual AI Backend Support** - watsonx.ai (cloud) or Ollama (local)
- ✅ **Smart Caching** - SHA-256 based, persistent across sessions
- ✅ **Privacy-First** - Local file scanning, you control data flow
- ✅ **50+ Languages** - Rust, Python, JavaScript, TypeScript, Go, and more
- ✅ **Beautiful TUI** - Split-pane interface with keyboard navigation
- ✅ **Zero Config** - Interactive setup on first run
- ✅ **Fast** - Async processing, instant cache retrieval
- ✅ **Robust** - Comprehensive error handling with helpful messages
- ✅ **Cross-Platform** - Works on Linux, macOS, and Windows

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

## 🙏 Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Ratatui](https://ratatui.rs/) - Terminal UI framework
- [Tokio](https://tokio.rs/) - Async runtime
- [IBM watsonx.ai](https://www.ibm.com/watsonx) - Enterprise AI platform
- [Ollama](https://ollama.ai) - Local AI runtime

---
