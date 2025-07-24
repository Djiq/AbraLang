# AbraLang

AbraLang is a statically-typed programming language written from scratch in Rust. It features its own compiler and interpreter, designed for simplicity and performance.
Features

 - Statically-Typed: Catch errors at compile-time, not runtime.

 - Simple, Indentation-based Syntax: A clean and intuitive syntax inspired by modern languages, using indentation for block scoping.

 - Rust-Powered: Built with Rust for safety, speed, and concurrency.

 - Compiler & Interpreter: Includes both a compiler for ahead-of-time compilation and an interpreter for quick scripting and development.

## Getting Started
Installation

Instructions on how to install AbraLang will go here once binaries are available.
## Usage

Here is a simple "Hello, World!" program in AbraLang:

    // hello_world.abra
    func sayHello(name: string) 
        print("Hello, " + name + "!")
        
    func main() -> int:
        sayHello("World")
        return 0

To run the program using the interpreter:

    abra run hello_world.abra

## Language Tour
### Variables and Types

    let name: string = "Abra"
    let version: float = 0.1
    let isAwesome: bool = true

### Functions

    func add(a: int, b: int) -> int:
        return a + b

let result = add(5, 7) // result is 12

## Building from Source

To build AbraLang from the source code, you'll need to have the Rust toolchain installed.

    Clone the repository:

    git clone https://github.com/Djiq/AbraLang.git
    cd AbraLang

    Build the project:

    cargo build --release

    Run the executable:
    The binary will be located at target/release/abra.

    ./target/release/abra --help

## Contributing

Contributions are welcome! If you'd like to contribute to AbraLang, please feel free to fork the repository and submit a pull request.

If you find a bug or have a feature request, please open an issue on the GitHub repository.
License

This project is licensed under the MIT License - see the LICENSE.md file for details.
