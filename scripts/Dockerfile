FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /node
COPY . .
#RUN cargo build --release

FROM docker.io/library/ubuntu:22.04

# Copy the node binary from builder stage.
COPY --from=builder /node/target/release/peaq-node /usr/local/bin
COPY --from=builder /node/node/src/chain-specs/  /node/src/chain-specs


RUN useradd -m -u 1000 -U -s /bin/sh -d /peaq peaq && \
  mkdir -p /chain-data /peaq/.local/share && \
  chown -R peaq:peaq /chain-data && \
  ln -s /chain-data /peaq/.local/share/node && \
  rm -rf /usr/bin /usr/sbin && \
  # check if executable works in this container
  /usr/local/bin/peaq-node --version

USER peaq

EXPOSE 9933 30333 9944 9615
VOLUME ["/chain-data"]
ENTRYPOINT ["/usr/local/bin/peaq-node"]