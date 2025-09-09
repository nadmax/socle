#!/bin/bash

set -e

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

if ! command -v buildah &>/dev/null; then
    error "buildah is not installed. Please install it first."
fi

if ! command -v podman &>/dev/null; then
    error "podman is not installed. Please install it first."
fi

if [[ -z "$GITHUB_TOKEN" ]]; then
    error "GITHUB_TOKEN environment variable is required"
fi

GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_TAG=$(git describe --tags --exact-match 2>/dev/null || echo "")
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

TAGS=()
TAGS+=("${REGISTRY}/${IMAGE_NAME}:${GIT_COMMIT}")

if [[ -n "$GIT_TAG" ]]; then
    TAGS+=("${REGISTRY}/${IMAGE_NAME}:${GIT_TAG}")
    if [[ "$GIT_TAG" =~ ^v?([0-9]+\.[0-9]+)\.[0-9]+$ ]]; then
        MAJOR_MINOR="${BASH_REMATCH[1]}"
        TAGS+=("${REGISTRY}/${IMAGE_NAME}:${MAJOR_MINOR}")
    fi
fi

if [[ "$GIT_BRANCH" == "master" || "$GIT_BRANCH" == "main" ]]; then
    TAGS+=("${REGISTRY}/${IMAGE_NAME}:latest")
fi

if [[ "$GIT_BRANCH" != "master" && -z "$GIT_TAG" ]]; then
    BRANCH_TAG=$(echo "$GIT_BRANCH" | sed 's/[^a-zA-Z0-9._-]/-/g')
    TAGS+=("${REGISTRY}/${IMAGE_NAME}:${BRANCH_TAG}")
fi

TAGS+=("${REGISTRY}/${IMAGE_NAME}:${TIMESTAMP}")

log "Building image with tags: ${TAGS[*]}"

log "Logging in to ${REGISTRY}..."
echo "$GITHUB_TOKEN" | podman login "$REGISTRY" --username "$(whoami)" --password-stdin

log "Creating build container..."
CONTAINER=$(buildah bud -f "${BUILD_CONTEXT}/Dockerfile" -t "$IMAGE_NAME" "${BUILD_CONTEXT}")

buildah config --label "org.opencontainers.image.source=https://github.com/nadmax/socle" "$CONTAINER"
buildah config --label "org.opencontainers.image.description=Socle Discord Bot" "$CONTAINER"
buildah config --label "org.opencontainers.image.revision=${GIT_COMMIT}" "$CONTAINER"
buildah config --label "org.opencontainers.image.created=$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$CONTAINER"

log "Committing image..."
IMAGE_ID=$(buildah commit "$CONTAINER" "${TAGS[0]}")

for tag in "${TAGS[@]:1}"; do
    log "Tagging image as $tag"
    buildah tag "$IMAGE_ID" "$tag"
done

for tag in "${TAGS[@]}"; do
    log "Pushing $tag..."
    buildah push "$tag"
done

log "Cleaning up..."
buildah rm "$CONTAINER"

log "Build and push completed successfully!"
log "Image ID: $IMAGE_ID"
log "Tags pushed: ${TAGS[*]}"
