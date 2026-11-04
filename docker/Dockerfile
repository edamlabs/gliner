# syntax=docker/dockerfile:1.28@sha256:bb22d9815c728170f72750f4e5b0d672e06176142e1d602c7e66c050100b7e5b
# The gliner service (:5800). Build context is the repository root: `make image`.
# No model inside and nothing downloaded at run time: mount an artifact dir (`make export`) read-only at /models.

##### BASES #####
ARG CHEF=lukemathwalker/cargo-chef:latest-rust-1.99-slim-trixie@sha256:c7b109b68df2196f7a8e5f370c4dfd5e4b5b98151f56c2a728887610dc8d1632
ARG RUNTIME=gcr.io/distroless/cc-debian13:nonroot@sha256:08476c244888bfb425bae1ae4dd7c17f093e972aab8426883d97f3c6f280f7c1

##### TOOLCHAIN #####
# g++ brings libstdc++ for the statically linked ONNX Runtime.
FROM ${CHEF} AS chef
WORKDIR /src
ENV CARGO_TARGET_DIR=/src/target
RUN apt-get update \
 && apt-get install -y --no-install-recommends g++ \
 && rm -rf /var/lib/apt/lists/*

##### RECIPE #####
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

##### BUILD #####
# ONNX Runtime is fetched prebuilt by `ort` here, at build time, and linked statically.
FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked --bin gliner \
 && install -D target/release/gliner /out/gliner

##### RUNTIME #####
# Distroless: libc, libstdc++ and certificates only; no shell, no package manager; uid 65532.
FROM ${RUNTIME}
COPY --from=builder /out/gliner /usr/local/bin/gliner
ENV GLINER_MODEL__DIR=/models \
    GLINER_SERVER__LOG__FORMAT=json
USER nonroot
EXPOSE 5800
# No shell, no curl: the binary probes its own `/ready` (healthy once the model is loaded).
HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=3 CMD ["/usr/local/bin/gliner", "probe"]
ENTRYPOINT ["/usr/local/bin/gliner"]
CMD ["serve"]
