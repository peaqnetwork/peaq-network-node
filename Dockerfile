# This is an example build stage for the node template. Here we create the binary in a temporary image.

# This is a base image to build substrate nodes
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /node
COPY . .
RUN cargo build --release

# This is the 2nd stage: a very small image where we copy the binary."
FROM docker.io/library/ubuntu:22.04
LABEL description="Multistage Docker image for Substrate Node Template" \
  image.type="builder" \
  image.authors="you@email.com" \
  image.vendor="Substrate Developer Hub" \
  image.description="Multistage Docker image for Substrate Node Template" \
  image.source="https://github.com/substrate-developer-hub/substrate-node-template" \
  image.documentation="https://github.com/substrate-developer-hub/substrate-node-template"

# Copy the node binary.
COPY --from=builder /node/target/release/peaq-node /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /peaq peaq && \
  mkdir -p /chain-data /peaq/.local/share && \
  chown -R peaq:peaq /chain-data && \
  ln -s /chain-data /peaq/.local/share/node && \
  # unclutter and minimize the attack surface
  rm -rf /usr/bin /usr/sbin && \
  # check if executable works in this container
  /usr/local/bin/peaq-node --version

USER peaq

EXPOSE 30333 9933 9944 9615
VOLUME ["/chain-data"]

ENTRYPOINT ["/usr/local/bin/peaq-node"]
