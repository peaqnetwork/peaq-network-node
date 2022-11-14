# This is an example build stage for the node template. Here we create the binary in a temporary image.

# This is a base image to build substrate nodes
FROM docker.io/paritytech/ci-linux:production as builder
#FROM ghcr.io/peaqnetwork/peaq-node-builder:agung-build as builder

#USER root
WORKDIR /opt/network/
COPY . .
RUN cargo build --release
RUN ls -al target/release/peaq-node

# This is the 2nd stage: a very small image where we copy the binary."
FROM ubuntu:20.04
LABEL description="Multistage Docker image for peaq Node" \
  image.type="builder" \
  image.authors="info@@eotlabs.io" \
  image.description="Multistage Docker image for peaq Node" \
  image.source="https://github.com/peaqnetwork/peaq-network-node" \
  image.documentation="https://github.com/peaqnetwork/peaq-network-node"

# Copy the node binary.
COPY --from=builder /target/release/peaq-node /opt/network/
#/usr/local/bin/peaq-node
RUN useradd -m -u 1000 -U -s /bin/sh -d /node-dev node-dev && \
  mkdir -p /chain-data /node-dev/.local/share && \
  chown -R node-dev:node-dev /chain-data && \
  ln -s /chain-data /node-dev/.local/share/peaq-node && \
  # unclutter and minimize the attack surface
  rm -rf /usr/bin /usr/sbin && \
  # check if executable works in this container
  /usr/local/bin/peaq-node --version

USER node-dev

EXPOSE 30333 9933 9944 9615
VOLUME ["/chain-data"]

ENTRYPOINT ["/usr/local/bin/peaq-node"]
