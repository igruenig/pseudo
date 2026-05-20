# Pseudo

Pseudo is a local-first desktop app for pseudonymizing confidential text. It uses a Tauri shell, a React frontend, Rust backend code, deterministic email/URL detectors, and an in-process local LLM through `llama.cpp`.

The app does not send document text to a server. The only expected network use during normal setup is downloading the local GGUF model file.

## Current Model

The default model target is:

- `SmolLM3-3B Q4_K_M`
- Downloaded from `ggml-org/SmolLM3-3B-GGUF`
- Stored in the app data directory under `pseudo/models/`
- Approximate download size: 1.9 GB

For comparison testing, Qwen can still be selected with:

```sh
PSEUDO_MODEL=qwen3 npm run tauri:dev
```

You can also point at any local GGUF file:

```sh
PSEUDO_MODEL_PATH=/absolute/path/to/model.gguf npm run tauri:dev
```

## Prerequisites

Install these on every development machine:

- Node.js 20 or newer
- npm
- Rust stable, at least Rust 1.77
- CMake
- A C/C++ build toolchain

Install project dependencies:

```sh
npm install
```

## Run Locally

Start the desktop app in development mode:

```sh
npm run tauri:dev
```

On first run, use the in-app download button to download the model. After the model is downloaded, click Analyze to load it and run local detection.

## Build For macOS

macOS builds must be produced on macOS.

Install Apple build tools:

```sh
xcode-select --install
```

Then build:

```sh
npm install
npm run tauri:build
```

The wrapper script sets the macOS deployment target and C++ compatibility flags needed by the local model runtime.

Build outputs are written under:

```text
src-tauri/target/release/bundle/
```

Common outputs include a `.dmg` and/or `.app`, depending on the host and Tauri bundler configuration.

## Build For Windows

Windows builds should be produced on Windows. Cross-compiling the Tauri app from macOS is not the intended path.

Install the required tools:

1. Install Node.js 20 or newer:
   - https://nodejs.org/

2. Install Rust and Cargo:
   - https://rustup.rs/

   Open a new PowerShell after installation and verify:

   ```powershell
   rustc --version
   cargo --version
   ```

   Rustup should install the MSVC toolchain by default on Windows. If needed, set it explicitly:

   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```

3. Install Visual Studio 2022 Build Tools:
   - https://visualstudio.microsoft.com/visual-cpp-build-tools/

   In the installer, select:

   ```text
   Desktop development with C++
   ```

   Make sure these components are included:

   - MSVC v143 build tools
   - Windows 10 or Windows 11 SDK
   - CMake tools for Windows

4. Install LLVM for `libclang`.

   `llama-cpp-sys-2` uses Rust `bindgen`, which needs `libclang.dll` during the build. Install LLVM from:

   - https://github.com/llvm/llvm-project/releases

   You can also install it with winget:

   ```powershell
   winget install LLVM.LLVM
   ```

   During installation, enable the option to add LLVM to `PATH` if offered. Then open a new PowerShell and verify:

   ```powershell
   clang --version
   ```

   Also verify that `libclang.dll` exists:

   ```powershell
   Test-Path "C:\Program Files\LLVM\bin\clang.exe"
   Test-Path "C:\Program Files\LLVM\bin\libclang.dll"
   ```

   If `clang` is not found after installation, add LLVM to the current PowerShell session:

   ```powershell
   $env:Path = "C:\Program Files\LLVM\bin;$env:Path"
   ```

   Then verify:

   ```powershell
   clang --version
   ```

   Set `LIBCLANG_PATH` to the LLVM `bin` directory for the current PowerShell:

   ```powershell
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   ```

   To make both paths persistent for your Windows user:

   ```powershell
   setx PATH "C:\Program Files\LLVM\bin;%PATH%"
   setx LIBCLANG_PATH "C:\Program Files\LLVM\bin"
   ```

   Close and reopen PowerShell after running `setx`.

5. Install the WebView2 Runtime if it is not already present:
   - https://developer.microsoft.com/microsoft-edge/webview2/

Then build from PowerShell:

```powershell
npm install
npm run tauri:build
```

Build outputs are written under:

```text
src-tauri\target\release\bundle\
```

Common outputs include `.msi` and/or `.exe` installers, depending on the Tauri bundler target generated on the machine.

### Windows Build Troubleshooting

If the build fails with:

```text
icons/icon.ico not found; required for generating a Windows Resource file during tauri-build
```

the Tauri icon set is missing. Regenerate it from the source icon:

```powershell
npm run tauri -- icon icons/icon.png
```

Run that command from the repository root. The wrapper runs Tauri from `src-tauri/app`, so `icons/icon.png` is the correct path.

If the build fails with:

```text
failed to run custom build command for `llama-cpp-sys-2`
```

look for the first concrete error above or below that line. Common causes:

- `cargo` or `rustc` is not on `PATH`: install Rust with rustup and open a new PowerShell.
- MSVC tools are missing: install Visual Studio Build Tools with "Desktop development with C++".
- CMake is missing: include "CMake tools for Windows" in Visual Studio Build Tools, or install CMake separately.
- `libclang` is missing: install LLVM and set `LIBCLANG_PATH` to `C:\Program Files\LLVM\bin`.

After changing toolchain installs or environment variables, close and reopen PowerShell before rebuilding.

## Checks

Frontend build:

```sh
npm run build
```

Rust tests:

```sh
cd src-tauri
cargo test
```

Local model smoke test, if a GGUF model is available:

```sh
cd src-tauri
cargo test -p pseudo-app model_runtime::tests::detects_with_local_model -- --ignored --nocapture
```

## Useful Environment Variables

```sh
# Select the built-in Qwen candidate instead of the default SmolLM3 candidate.
PSEUDO_MODEL=qwen3

# Use an explicit local GGUF file.
PSEUDO_MODEL_PATH=/absolute/path/to/model.gguf
```

## Project Layout

```text
src/                         React frontend
src-tauri/app/               Tauri app crate and local model runtime
src-tauri/pseudo-core/       Pure Rust pseudonymization core
src-tauri/pseudo-cli/        CLI wrapper around pseudo-core
scripts/tauri.mjs            Tauri command wrapper
```
