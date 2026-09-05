# Web runtime image. Do not pass secrets as build args or copy .env files.
# Copy listed files only so a local node_modules or .next directory cannot enter the image.
FROM node:20-bookworm-slim AS deps

WORKDIR /app

COPY apps/web/package.json apps/web/package-lock.json apps/web/.npmrc ./
RUN npm ci

FROM node:20-bookworm-slim AS builder

WORKDIR /app

ENV NEXT_TELEMETRY_DISABLED=1 \
    NEXT_PUBLIC_API_BASE_URL=/api/v1

COPY --from=deps /app/node_modules ./node_modules
COPY apps/web/package.json apps/web/package-lock.json apps/web/.npmrc ./
COPY apps/web/next.config.ts apps/web/tsconfig.json apps/web/postcss.config.mjs ./
COPY apps/web/src ./src

RUN npm run build

FROM node:20-bookworm-slim AS runtime

WORKDIR /app

RUN useradd --system --uid 10001 --create-home --shell /usr/sbin/nologin nextjs

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    NEXT_PUBLIC_API_BASE_URL=/api/v1 \
    API_ORIGIN=http://127.0.0.1:3000 \
    PORT=3000 \
    HOSTNAME=0.0.0.0

COPY apps/web/package.json apps/web/package-lock.json apps/web/.npmrc ./
COPY apps/web/next.config.ts ./
RUN npm ci --omit=dev && npm cache clean --force

COPY --from=builder --chown=nextjs:nextjs /app/.next ./.next
RUN chown -R nextjs:nextjs /app

USER nextjs

EXPOSE 3000

HEALTHCHECK --interval=15s --timeout=5s --start-period=20s --retries=5 \
    CMD node -e "require('http').get('http://127.0.0.1:3000', (response) => process.exit(response.statusCode < 500 ? 0 : 1)).on('error', () => process.exit(1))"

CMD ["npx", "next", "start", "-H", "0.0.0.0", "-p", "3000"]
