#!/bin/bash
set -e

echo "Building Docker image for testing (this might take a few minutes as it installs Homebrew)..."

cat << 'EOF' > Dockerfile.test
FROM rust:slim-bullseye

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    sudo curl git build-essential procps file gcc

# Create a non-root user (Homebrew cannot run as root)
RUN useradd -m -s /bin/bash testuser && \
    usermod -aG sudo testuser && \
    echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER testuser
ENV USER=testuser
WORKDIR /home/testuser

# Install Homebrew quietly
RUN NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Setup Homebrew environment
ENV PATH="/home/linuxbrew/.linuxbrew/bin:${PATH}"

RUN mkdir -p /home/testuser/app && chown testuser:testuser /home/testuser/app
WORKDIR /home/testuser/app
COPY --chown=testuser:testuser . .

# Run the test
CMD echo "Building bkzyn..." && \
    sudo rm -rf target && \
    cargo build && \
    echo "\n--- Running bkzyn setup ---" && \
    ./target/debug/bkzyn --verbose setup && \
    echo "\n--- Verification ---" && \
    echo "1. Checking ZDOTDIR in /etc/zshenv:" && \
    cat /etc/zshenv && \
    echo "\n2. Checking XDG_CONFIG_HOME:" && \
    ls -la ~/.config
EOF

docker build -t bkzyn-test -f Dockerfile.test .
rm Dockerfile.test

echo "Running the test container..."
docker run --rm bkzyn-test
