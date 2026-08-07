<img width="975" height="445" alt="image" src="docs/assets/Frame 1.svg" />

<br>

>**The Gateway to Contextual Configs.**
---

## 📌 Overview

**CTEX** (short for *context*) is an embedded, high-performance systems and configuration language that aims to turn static settings into active, contextual execution surfaces.

Traditional configuration formats like TOML or YAML force you to specify static constants. CTEX takes a fundamentally different approach~

1. **Host-Exposed API:** CTEX sits directly alongside host application source code, doubling as both a runtime config engine and a lightweight plugin system.
2. **Context-Driven Computation:** Instead of static values, a `.ctx` program takes an input context (`%ictx`), performs computation in real time on it, and yields a computed output context (`%octx`).
3. **Low-Level Native Integration:** Designed with explicit _memory layout_ handling in mind, CTEX is projected to work natively with Protobuf, raw bytes, or raw host-defined struct layouts (ideal for low-level interaction with languages like C that allow flexible byte-to-struct mapping via explicit interfaces).

Because CTEX compiles right alongside the host source, performance is paramount. Currently, it features a custom **mmap-backed, SIMD-accelerated lexer** and a recursive-descent parser capable of scanning nearly a million tokens in tens of milliseconds.

---

## 🛠️ Architectural Overview

### Top-Level Forms
CTEX programs rely on two primray top-level constructs:
* **`INIT` (Optional):** For setting up the I/O contract: payload formats, struct schemas, core signatures and so on and so forth. These must be preconfigured before execution reaching ENTRY.
* **`ENTRY`:** As the name suggests, the program entry point.

### Reserved Registers
Rather than relying on the standard parameter/argument model, CTEX follows a niche, yet highly accommodating of an ideology, employing registers, following which, it claims certain identifiers as _reserved_ registers:
* **`%ictx` / `%octx`**: Input/Output "Context" Buffer Registers. `%ictx` holds the initial data that is passed to the CTEX program as context, while `%octx` is the register, intended to be mutated throughout the length of the program and eventually returned to the host language.
* **`%ictx_F` / `%octx_F`**: Format definitions (e.g., Protobuf, raw bytes, or custom host byte stream layouts).
* **`%ictx_T` / `%octx_T`**: Schemas for `%ictz` / `%octx` respectively.
* **`%argv` / `%retv`**: Default argument and return register channels.
  * Passing arguments via `fn(xyz)` is only shorthand for pushing `xyz` into `%argv` and subsequently running the function denoted by `fn`.
  * `return xyz`, apart from returning the value at hand, pushes `xyz` into `%retv`.

### Execution Modes
* **Standard Mode:** Executes alongside standard host I/O.
* **Isolated Mode:** Blocks host `stdio` immediately after populating `%ictx`, only unblocking it to reemit `%octx` at the end of the program.

---

## ⚡ Performance Benchmarks

CTEX - Tokenization + AST GEN (Release Build):

| Input File / Scale | Tokens | Lexer (mmap + SIMD) | Parser (AST) |
| :--- | :--- | :--- | :--- |
| **`foc.ctx` (~3 KB)** | 646 | ~100–280 µs | ~25–50 µs |
| **Scaled Showcase (~355 KB / ~15k lines)** | ~92,000 | ~8.5 ms | ~2.4–4.5 ms |
| **Stress Test (~3.5 MB / ~150k lines)** | ~922,000 | ~54 ms | ~22–23 ms |

---

## 💻 Installation & Dependencies

### 1. Installing the Rust Toolchain

CTEX requires a recent Rust toolchain, namely (**Edition 2024**), which can be installed using `rustup`:

```bash
# Install rustup (Linux / macOS)
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh

# Source environment
source "$HOME/.cargo/env"

# Ensure you have the latest toolchain installed
rustup update

```

_(On Windows, download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs/).)_

### 2. Fetching Dependencies + Build

All Rust crate dependencies (`memmap2`, `wide`, `display_tree`) are fetched automatically by Cargo upon build.

Bash

```
# Clone the repo
git clone git@github.com:hurshbajaj/Ctex.git
cd Ctex

# Build in release mode
cargo build --release

```

## 🚀 Quick Start

Run the parser on the provided showcase file:

Bash

```
# Parse a ctx file (prints execution time diagnostics)
cargo run --release -- foc.ctx

# Lex + parse with full Token Stream & AST dump
cargo run --release -- --dump foc.ctx

# Or use the included helper script
./run.sh path/to/file.ctx

```

## 📁 Repository Layout

```
src/
├── main.rs              # CLI Driver (lexing, parsing, --dump flag)
└── frontend/
    ├── lexer.rs         # Memory-mapped, chunked lexer
    ├── simd.rs          # SIMD acceleration via `wide` crate
    ├── tokens.rs        # Token typings and definitions
    ├── keywords.rs      # Keyword and directive lookups
    └── ast.rs           # Recursive-descent AST parser

```

## 📝 Author's Note

I'd like to start off by clarifying that the emojis / comments throughout are very much intended. Apart from rare debugging on small, isolated, fragmented bits, and refining this README (Written by me) I have made it a point not to use or rely on AI throughout the development of the project. Certainly never on whole files or big chunks. The architectural design, register model, experimental paradigm, and performance optimizations are, I assure you, my very own.

— **Hursh B**

## 🗺️ Roadmap & Status

This repository is currently a **checkpoint**:

-   [x] Memory-mapped SIMD Lexer
    
-   [x] Recursive-descent AST Parser
    
-   [x] Full AST dumping & visualization tool
    
-   [ ] Intermediate Representation (IR) generation _(Ongoing)_
    
-   [ ] Diagnostic & Error reporting system _(Ongoing)_
    
-   [ ] Runtime execution engine & Host embedding layer
