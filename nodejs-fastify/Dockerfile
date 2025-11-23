# Builder stage: install deps and compile TypeScript
FROM node:20-alpine AS builder
WORKDIR /app

# Install build-time dependencies (including devDeps for TypeScript)
COPY package*.json ./
RUN npm ci

# Copy sources and build
COPY tsconfig.json ./
COPY index.ts ./
RUN npm run build

# Remove devDependencies to keep node_modules production-only
RUN npm prune --production

# Runtime stage: smaller image with only production artifacts
FROM node:20-alpine AS runner
WORKDIR /app

# Environment
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

# Copy built files and production node_modules from builder
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package*.json ./

EXPOSE 3000

# Use node to run the compiled JS directly
CMD ["node", "dist/index.js"]
