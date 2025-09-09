#!/bin/bash

set -exo pipefail

REGISTRY="ghcr.io"
IMAGE_NAME="nadmax/socle"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"
BUILD_CONTEXT="${BUILD_CONTEXT:-.}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

command -v buildah >/dev/null || error "buildah is not installed"
command -v podman >/dev/null || error "podman is not installed"
[[ -n "$GITHUB_TOKEN" ]] || error "GITHUB_TOKEN is required"

VERSION=$(jq -r '.version' package.json)
TAGS=()

if [[ -n "$VERSION" ]]; then
    TAGS+=("${REGISTRY}/${IMAGE_NAME}:${VERSION}")
    if [[ "$VERSION" =~ ^v?([0-9]+\.[0-9]+)\.[0-9]+$ ]]; then
        MAJOR_MINOR="${BASH_REMATCH[1]}"
        TAGS+=("${REGISTRY}/${IMAGE_NAME}:${MAJOR_MINOR}")
    fi
fi

if [[ "$GIT_BRANCH" == "master" ]]; then
    TAGS+=("${REGISTRY}/${IMAGE_NAME}:latest")
fi

log "Building image with tags: ${TAGS[*]}"

log "Logging in to ${REGISTRY}..."
echo "$GITHUB_TOKEN" | podman login "$REGISTRY" --username "$(whoami)" --password-stdin

log "Building image..."
IMAGE_ID=$(
    buildah bud \
        --file "${BUILD_CONTEXT}/Dockerfile" \
        --tag "${TAGS[0]}" \
        --label "org.opencontainers.image.source=https://github.com/nadmax/socle" \
        --label "org.opencontainers.image.description=Socle Discord Bot" \
        --label "org.opencontainers.image.revision=${VERSION}" \
        --label "org.opencontainers.image.created=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --quiet \
        "${BUILD_CONTEXT}"
)
IMAGE_ID=$(echo "$IMAGE_ID" | tr -d "'[:space:]")

for tag in "${TAGS[@]:1}"; do
    log "Tagging image as $tag"
    buildah tag "$IMAGE_ID" "$tag"
done

for tag in "${TAGS[@]}"; do
    log "Pushing $tag..."
    buildah push "$tag"
done

log "Build and push completed successfully!"
log "Image ID: $IMAGE_ID"
log "Tags pushed: ${TAGS[*]}"
