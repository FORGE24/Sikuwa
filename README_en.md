# Sikuwa Tool Documentation

## Introduction

Sikuwa is a Python project packaging tool based on Nuitka, focused on providing simple and efficient cross-platform compilation solutions. It converts Python projects into standalone executable files through configuration management and automated workflows, supporting distribution on Windows, Linux, and macOS platforms.

### Core Advantages
- **Cross-platform Support**: Compatible with Windows, Linux, and macOS systems simultaneously
- **Flexible Configuration**: Customize compilation parameters through TOML configuration files to meet different project requirements
- **Dual Environment Checks**: Automatically detect system dependencies and compilation environments for early problem troubleshooting
- **Dual Usage Modes**: Support both pre-compiled version (standalone toolchain) and source code version (Python library)
- **Detailed Logs and Manifest**: Generate build logs and output manifests for easy version management and distribution

## Update Notes

### v1.3.0 Key Features
1. **Smart Cache System**
   - High-performance C++ LRU caching algorithm
   - Python wrapper interface for cross-platform calls
   - Pure Python fallback mechanism for compatibility
   - Intelligent cache key generation based on file content and build parameters
   - Deep integration with the build process for automatic cache management
   - Support for forced rebuild and cache cleanup functions

2. **Performance Optimization**
   - First build: ~30 seconds, cache hit: ~1.5 seconds
   - Significantly reduced repeated build times
   - Low memory usage with efficient cache management

### v1.2.0 Key Features
1. **Basic Function Implementation**
   - Complete project initialization and configuration management
   - Multi-platform compilation support (Windows/Linux/macOS)
   - Environment checking and dependency verification
   - Automatic build manifest generation

2. **Core Optimizations**
   - Comprehensive logging system with detailed mode for tracking compilation processes
   - Automatic resource file copying mechanism
   - Build caching and forced rebuild functionality
   - Command-line interaction experience optimization

3. **Compatibility Improvements**
   - Support for Python 3.7+ versions
   - Compatible with latest Nuitka compilation options
   - Adapted to mainstream C compilers (MSVC/GCC/MinGW)

## Pre-compiled Version Usage (Standalone Toolchain)

The pre-compiled version can be used as a standalone toolchain without installing a Python environment. Simply add it to the system PATH for global access.

### Installation Configuration
1. Download the pre-compiled package for your platform from official channels
2. Extract to a local directory (e.g., `C:\sikuwa` or `~/sikuwa`)
3. Add the extraction directory to the system environment variable `PATH`
4. Verify installation:
   ```bash
   sikuwa --version
   ```

### Command Reference

#### 1. Initialize Configuration
```bash
# Create default configuration file (sikuwa.toml)
sikuwa init

# Create custom configuration file
sikuwa init -o my_config.toml

# Force overwrite existing configuration file
sikuwa init --force
```

#### 2. Build Project
```bash
# Build all platforms (default configuration)
sikuwa build

# Build specific platform
sikuwa build -p windows
sikuwa build -p linux
sikuwa build -p macos

# Use verbose output mode (view compilation process)
sikuwa build -v

# Use specified configuration file
sikuwa build -c my_config.toml

# Force rebuild (ignore cache)
sikuwa build --force
```

#### 3. View Project Information
```bash
# Display current project configuration information
sikuwa info

# Display information from specified configuration file
sikuwa info -c my_config.toml
```

#### 4. Environment Check
```bash
# Check system environment and dependencies
sikuwa doctor
```

#### 5. Clean Build Files
```bash
# Delete output directory and build cache
sikuwa clean
```

#### 6. View Help
```bash
# Display general help
sikuwa --help

# Display help for specific command
sikuwa build --help

# Display help for configuration file
sikuwa help config
```

#### 7. Version Information
```bash
# Display version information
sikuwa version
```

## Source Code Version Usage (Python Library)

The source code version can be integrated into other projects as a Python library, implementing compilation functionality through API calls.

### Installation Method
```bash
# Install from source
pip install .

# Install in development mode
pip install -e .
```

### Core API Usage Examples

#### 1. Basic Build Process
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

# Load configuration
config = ConfigManager.load_config("sikuwa.toml")

# Initialize builder
builder = SikuwaBuilder(config, verbose=True)

# Execute build (all platforms)
builder.build()

# Execute build (specific platform)
builder.build(platform="windows")
```

#### 2. Custom Configuration
```python
from sikuwa.config import BuildConfig, NuitkaOptions
from sikuwa.builder import SikuwaBuilder

# Create custom configuration
nuitka_options = NuitkaOptions(
    standalone=True,
    onefile=True,
    enable_console=False,
    windows_icon="app_icon.ico"
)

config = BuildConfig(
    project_name="my_app",
    main_script="main.py",
    version="1.0.0",
    platforms=["windows", "linux"],
    nuitka_options=nuitka_options,
    resources=["data/*"]
)

# Execute build
builder = SikuwaBuilder(config)
builder.build(force=True)
```

#### 3. Clean Build Files
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder.clean()  # Clean output directory and build directory
```

#### 4. Generate Build Manifest
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder._generate_manifest()  # Generate build manifest file
```

## Compilation Guide

### Prerequisites
- Python 3.7 or higher
- System compiler:
  - Windows: MinGW-w64 (8.1.0+) or MSVC (2019+)
  - Linux: GCC (7.3+)
  - macOS: Xcode Command Line Tools
- Dependencies:
  ```bash
  pip install nuitka click tomli tomli_w
  ```

### Compilation Steps

1. **Prepare Configuration File**
   ```bash
   # Generate default configuration
   sikuwa init
   
   # Edit configuration file (key settings)
   # Project name, entry file, target platforms, Nuitka options, etc.
   ```

2. **Check Environment**
   ```bash
   sikuwa doctor
   ```
   Ensure all checks show `[OK]`, resolve any `[FAIL]` items

3. **Execute Compilation**
   ```bash
   # Basic compilation (all platforms)
   sikuwa build
   
   # Single platform compilation
   sikuwa build -p windows
   
   # Detailed mode compilation (for debugging)
   sikuwa build -v
   ```

4. **View Output**
   After successful compilation, output files are located in the configured `output_dir` (default `dist` directory), categorized by platform:
   - Windows: `dist/project-name-windows/`
   - Linux: `dist/project-name-linux/`
   - macOS: `dist/project-name-macos/`

5. **Verify Results**
   The build manifest file `dist/build_manifest.json` contains information about all output files:
   - Project name and version
   - Build time
   - Output file paths and sizes for each platform

## Bootstrap Guide

Bootstrapping refers to using the Sikuwa tool to compile its own source code, generating a standalone Sikuwa executable file.

### Bootstrap Steps

1. **Obtain Source Code**
   ```bash
   git clone https://github.com/yourusername/sikuwa.git
   cd sikuwa
   ```

2. **Prepare Environment**
   ```bash
   # Install dependencies
   pip install -r requirements.txt
   
   # Check environment
   python -m sikuwa doctor
   ```

3. **Configure Bootstrap Parameters**
   ```bash
   # Generate configuration file
   python -m sikuwa init
   
   # Edit configuration file (key settings)
   # Ensure the following configuration in sikuwa.toml
   ```
   ```toml
   [sikuwa]
   project_name = "sikuwa"
   main_script = "sikuwa/__main__.py"
   version = "1.3.0"
   platforms = ["windows"]
   
   [sikuwa.nuitka]
   standalone = true
   onefile = true
   follow_imports = true
   enable_console = true
   ```

4. **Execute Bootstrap Compilation**
   ```bash
   # Use source code version to compile itself
   python -m sikuwa build -v
   ```

5. **Verify Bootstrap Results**
   ```bash
   # Enter output directory
   cd dist/sikuwa-<current-platform>
   
   # Verify generated executable
   ./sikuwa --version  # Linux/macOS
   sikuwa.exe --version  # Windows
   ```

6. **Test Bootstrap Version**
   ```bash
   # Create test project
   mkdir test_bootstrap && cd test_bootstrap
   
   # Initialize project using bootstrap-generated tool
   ../dist/sikuwa-<current-platform>/sikuwa init
   
   # Create simple entry file
   echo 'print("Hello, Sikuwa!")' > main.py
   
   # Build test project
   ../dist/sikuwa-<current-platform>/sikuwa build
   ```

If all steps execute normally, the bootstrap is successful. The generated executable can be used as a standalone toolchain without depending on a Python environment.