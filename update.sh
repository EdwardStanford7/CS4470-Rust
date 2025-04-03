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
gh repo clone utah-cs4470-sp25/grader grader_temp
rm -rf grader
mkdir grader
cp -r grader_temp/* grader/
rm -rf grader_temp

echo "installing runtime"
gh repo clone utah-cs4470-sp25/runtime rt_temp
rm -rf rt
mkdir rt
cp -r rt_temp/* rt/
rm -rf rt_temp

cd rt && make && cd ..

echo "downloading JPLC and JPLI"
mkdir -p examples
rm -f examples/jpli examples/jplc grader/jplc

# Get the latest release tag using GitHub CLI
latest_release=$(gh release list --repo utah-cs4470-sp25/class --json tagName,isLatest -q '.[] | select(.isLatest==true) | .tagName')

# Construct the download URLs with proper variable interpolation
interpreter_url="https://github.com/utah-cs4470-sp25/class/releases/download/${latest_release}/jpli-macos"
compiler_url="https://github.com/utah-cs4470-sp25/class/releases/download/${latest_release}/jplc-macos"

curl -LO "$interpreter_url"
curl -LO "$compiler_url"

mv "jpli-macos" "jpli"
mv "jplc-macos" "jplc"
mv jpli examples/
mv jplc examples/ && cp examples/jplc grader/
chmod +x examples/jpli examples/jplc grader/jplc