#!/bin/bash

# Detect OS type
OS=$(uname)

if [ "$OS" = "Darwin" ]; then
  echo "Detected macOS"
  # Ensure nasm is installed via Homebrew
  brew list nasm &>/dev/null && echo "nasm already installed" || brew install nasm
  # Ensure GitHub CLI (gh) is installed via Homebrew
  command -v gh &>/dev/null && echo "gh already installed" || brew install gh
elif [ "$OS" = "Linux" ]; then
  echo "Detected Linux"
  # Update package lists
  sudo apt-get update
  # Ensure nasm is installed via apt-get
  dpkg -s nasm &>/dev/null && echo "nasm already installed" || sudo apt-get install -y nasm
  # Ensure GitHub CLI (gh) is installed via apt-get
  command -v gh &>/dev/null && echo "gh already installed" || sudo apt-get install -y gh
else
  echo "Unsupported OS: $OS"
  exit 1
fi

echo "installing grader"
rm -rf grader
gh repo clone utah-cs4470-sp25/grader grader

echo "installing runtime"
rm -rf rt
gh repo clone utah-cs4470-sp25/runtime rt

cd rt && make && cd ..
cd grader && make jplc && cd ..
