# syntax=docker/dockerfile:1

# ---- Build stage -----------------------------------------------------------
# Produce the static Vite bundle with pnpm.
FROM node:20-slim AS builder

# Enable pnpm via corepack (shipped with modern Node).
RUN corepack enable

WORKDIR /app

# Install dependencies against the lockfile for reproducible builds.
COPY package.json pnpm-lock.yaml* ./
RUN pnpm install --frozen-lockfile || pnpm install

# Copy sources and build. VITE_API_URL is baked in at build time; override with
# --build-arg VITE_API_URL=... to target a non-default API origin.
ARG VITE_API_URL=http://localhost:8080
ENV VITE_API_URL=${VITE_API_URL}
COPY . .
RUN pnpm build

# ---- Runtime stage ---------------------------------------------------------
# Serve the static assets with nginx (gzip + SPA fallback), running unprivileged.
FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime

# Custom config: gzip, long-lived asset caching, history-API SPA fallback.
COPY nginx.conf /etc/nginx/conf.d/default.conf

COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/ >/dev/null 2>&1 || exit 1
