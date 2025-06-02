FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json tsconfig.json ./
RUN npm ci
COPY ./ ./
RUN npm run build

FROM node:22-alpine
WORKDIR /app

COPY package*.json ./
RUN npm ci --production && \
    npm cache clean --force && \
    rm -rf /root/.npm /tmp/* /var/cache/apk/* /usr/share/man /usr/share/doc && \
    find node_modules -type f -name "*.md" -delete && \
    find node_modules -type f -name "*.ts" -delete && \
    find node_modules -type f -name "*.map" -delete && \
    find node_modules -type d -name "test" -o -name "__tests__" | xargs rm -rf

COPY --from=builder /app/build/ ./build/
COPY src/scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]