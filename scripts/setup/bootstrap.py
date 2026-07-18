#!/usr/bin/env python3

import os
import shutil
import subprocess
import sys
from pathlib import Path


class Colors:
    OK = "\033[92m"
    WARNING = "\033[93m"
    FAIL = "\033[91m"
    END = "\033[0m"


def print_step(message: str) -> None:
    print(f"{Colors.OK}[+] {message}{Colors.END}")


def print_info(message: str) -> None:
    print(f"[i] {message}")


def print_warning(message: str) -> None:
    print(f"{Colors.WARNING}[-] {message}{Colors.END}")


def print_error(message: str) -> None:
    print(f"{Colors.FAIL}[!] {message}{Colors.END}")
    sys.exit(1)


def check_command(command: str, name: str) -> None:
    if shutil.which(command) is None:
        print_error(f"{name} ({command}) was not found. Please install it and ensure it is available in PATH.")

    print(f"  - {name} found.")


def run_command(command: list[str], cwd: Path | None = None) -> None:
    try:
        subprocess.run(command, cwd=cwd, check=True)
    except subprocess.CalledProcessError as error:
        print_error(
            f"Command failed:\n"
            f"{' '.join(command)}\n"
            f"Exit Code: {error.returncode}"
        )


def main() -> None:
    root_dir = Path(__file__).resolve().parent.parent.parent
    os.chdir(root_dir)

    print_step(f"Workspace directory: {root_dir}")

    print_step("Checking required development tools...")

    tools = {
        "cargo": "Rust (Cargo)",
        "cmake": "CMake",
        "go": "Go",
        "npm": "Node.js (npm)",
        "python": "Python",
    }

    for command, name in tools.items():
        check_command(command, name)

    print_step("Preparing CMake build directory...")

    build_dir = root_dir / "build"
    build_dir.mkdir(exist_ok=True)

    run_command([
        "cmake",
        "-S",
        ".",
        "-B",
        str(build_dir),
    ])

    print_info("Setup completed successfully.")
    print_info("Run your preferred build system when you're ready to compile the engine.")


if __name__ == "__main__":
    main()
