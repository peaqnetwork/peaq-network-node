# Use a lightweight Node.js image
FROM node:18-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the config file into the image (optional if mounting instead)
COPY peaq.yaml .

# Default command (can be overridden)
CMD ["npx", "@acala-network/chopsticks", "--config=peaq.yaml"]
