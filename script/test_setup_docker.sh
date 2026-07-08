#!/bin/bash
set -e

echo "Starting bkzyn Linux development environment..."
echo "This will mount your local repository into the container."
echo "Compiling the CLI will not affect your Mac's target/ directory."
echo ""

# Bring up the container in detached mode
docker compose up -d --build bkzyn-linux-test

echo ""
echo "Container is running in the background!"
echo "To run tests or compile interactively, exec into the container with:"
echo "    docker compose exec bkzyn-linux-test bash"
echo ""
echo "Once inside, you can run:"
echo "    ./script/e2e_test.sh"
echo "    cargo build"
echo ""
echo "To stop the environment, run:"
echo "    docker compose down"
