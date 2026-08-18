# Docker Deployment

GhostMark ships with a multi-stage Dockerfile and a `docker-compose.yml` for instant self-hosted deployments.

## Quick Start

```bash
# Clone the repo
git clone https://github.com/kilopal/GhostMark.git
cd GhostMark

# Start the API server
docker compose up -d

# Verify it's running
curl http://localhost:8080/health
# → {"ok":true,"version":"0.4.0","engine":"GhostMark/Rust"}
```

## Architecture

The Docker image uses a **multi-stage build**:

| Stage | Base Image | Purpose |
|-------|-----------|---------|
| `builder` | `rust:1.88-slim` | Compiles the Rust workspace into a single static binary |
| `runtime` | `debian:bookworm-slim` | Runs only the ~5 MB binary — no Rust toolchain shipped |

The final image is typically **under 80 MB** — far smaller than Python-based alternatives.

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `GHOSTMARK_PORT` | `8080` | Host port to expose the API on |
| `RUST_LOG` | `info` | Logging level (`debug`, `info`, `warn`, `error`) |

### Custom Port

```bash
GHOSTMARK_PORT=3000 docker compose up -d
# API now available at http://localhost:3000
```

## API Endpoints

Once the container is running, the following endpoints are available:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Service health check |
| `POST` | `/clean/text` | Strip Unicode watermarks from text |
| `POST` | `/inspect/text` | Detect watermark characters in text |
| `POST` | `/clean/image` | Strip C2PA/EXIF metadata from images |

### Example: Clean Text

```bash
curl -X POST http://localhost:8080/clean/text \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello\u200Bworld"}'
```

### Example: Clean Image

```bash
# Encode image to base64 and send
curl -X POST http://localhost:8080/clean/image \
  -H "Content-Type: application/json" \
  -d "{\"file\": \"$(base64 < photo.jpg | tr -d '\n')\", \"name\": \"photo.jpg\"}"
```

## Resource Limits

The default `docker-compose.yml` sets conservative limits:

- **Memory:** 256 MB
- **CPU:** 1.0 core

These can be adjusted in `docker-compose.yml` under `deploy.resources.limits`.

## Health Checks

The container includes a built-in health check that pings `/health` every 30 seconds. Use `docker ps` to see the health status:

```bash
docker ps
# CONTAINER ID   IMAGE        STATUS                    PORTS
# abc123         ghostmark    Up 5m (healthy)           0.0.0.0:8080->8080/tcp
```

## Building Manually

If you prefer not to use Docker Compose:

```bash
# Build the image
docker build -t ghostmark .

# Run it
docker run -d --name ghostmark-api -p 8080:8080 ghostmark
```

## Stopping

```bash
docker compose down
```
