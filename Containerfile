# The Study App in a browser, for testing with throwaway databases.
#   podman compose up --build -d   → http://localhost:8080 with a seeded demo profile
#   podman compose down -v         → stops it and deletes the database
# The desktop app is unchanged. This builds crates/study-web and the same Svelte frontend.

# ---- Frontend ----
FROM docker.io/library/node:22-bookworm-slim AS frontend
WORKDIR /app
RUN npm install -g pnpm@10.28.0
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store pnpm install --frozen-lockfile
COPY index.html vite.config.ts svelte.config.js tsconfig.json ./
COPY src ./src
RUN pnpm build

# ---- Server ----
FROM docker.io/library/rust:1-slim-bookworm AS server
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The workspace lists the desktop shell, so cargo needs its manifest (it isn't built here).
COPY src-tauri/Cargo.toml ./src-tauri/Cargo.toml
RUN mkdir -p src-tauri/src && touch src-tauri/src/lib.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p study-web && cp target/release/study-web /usr/local/bin/study-web

# ---- Runtime ----
FROM docker.io/library/debian:bookworm-slim
# tzdata so "today" follows TZ below. PowerPoint previews need LibreOffice, which is big,
# so it's only added with --build-arg SLIDES=1.
ARG SLIDES=0
RUN apt-get update \
 && apt-get install -y --no-install-recommends tzdata $( [ "$SLIDES" = "1" ] && echo libreoffice-impress ) \
 && rm -rf /var/lib/apt/lists/*
COPY --from=server /usr/local/bin/study-web /usr/local/bin/study-web
COPY --from=frontend /app/dist /srv/study/dist
ENV STUDY_DB=/data/study.sqlite \
    STUDY_DIST=/srv/study/dist \
    STUDY_ADDRESS=0.0.0.0:8080 \
    TZ=Europe/Isle_of_Man
VOLUME /data
EXPOSE 8080
CMD ["study-web"]
