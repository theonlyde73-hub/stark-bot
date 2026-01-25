# StarkBot

A Rust-based bot with an Actix webserver, frontend login, and SQLite session storage.

## Local Development

### Prerequisites

- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- SQLite3 (usually pre-installed on Linux)

### Generate a Secret Key

```bash
# Using openssl (recommended)
openssl rand -base64 32

# Using /dev/urandom
head -c 32 /dev/urandom | base64

# Using uuid
cat /proc/sys/kernel/random/uuid
```

### Run Locally

```bash
# Copy environment template
cp .env.template .env

# Edit .env and set your SECRET_KEY
nano .env

# Run the server
cargo run -p stark-backend

# Or run directly with environment variables
SECRET_KEY=your-secret-key cargo run -p stark-backend
```

The server starts at `http://localhost:8080`

### Test Endpoints

```bash
# Health check
curl http://localhost:8080/health

# Login
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"secret_key":"your-secret-key"}'
```

## Local Docker Testing

### Build the Docker Image

```bash
docker build -t starkbot .
```

### Run with Docker

```bash
# Generate a secret key
SECRET_KEY=$(openssl rand -base64 32)

# Run the container
docker run -p 8080:8080 -e SECRET_KEY="$SECRET_KEY" starkbot

# Or run in detached mode
docker run -d -p 8080:8080 -e SECRET_KEY="$SECRET_KEY" --name starkbot starkbot

 

```


### Shut down docker when done

```


docker stop starkbot 

docker rm starkbot  

```





### Run with Persistent Database

```bash
# Create the db directory
mkdir -p ./.db

# Run with volume mount for persistent SQLite
docker run -p 8080:8080 \
  -e SECRET_KEY="$SECRET_KEY" \
  -e DATABASE_URL=/app/.db/stark.db \
  -v $(pwd)/.db:/app/.db \
  starkbot
```

### Test the Container

```bash
# Health check
curl http://localhost:8080/health

# Open in browser
xdg-open http://localhost:8080  # Linux
open http://localhost:8080      # macOS
```

### Stop and Clean Up

```bash
# Stop running container
docker stop starkbot

# Remove container
docker rm starkbot

# Remove image (optional)
docker rmi starkbot
```

## Deploy to DigitalOcean App Platform

### 1. Push to GitHub

```bash
git init
git add .
git commit -m "Initial commit"
git remote add origin git@github.com:yourusername/starkbot.git
git push -u origin main
```

### 2. Create App on DigitalOcean

1. Go to [DigitalOcean App Platform](https://cloud.digitalocean.com/apps)
2. Click **Create App**
3. Select **GitHub** and authorize access
4. Choose your `starkbot` repository
5. Select the branch (e.g., `main`)

### 3. Configure the App

DigitalOcean should auto-detect the Dockerfile. If not, manually configure:

- **Type**: Web Service
- **Source**: Dockerfile
- **HTTP Port**: 8080
- **Health Check Path**: `/health`

### 4. Set Environment Variables

In the App settings, add:

| Variable | Value |
|----------|-------|
| `SECRET_KEY` | *(generate with `openssl rand -base64 32`)* |
| `RUST_LOG` | `info` |
| `DATABASE_URL` | `/app/.db/stark.db` |

To encrypt the secret key:
1. Click on the `SECRET_KEY` variable
2. Check **Encrypt** to hide the value

### 5. Configure Persistent Storage (Optional)

For persistent SQLite data across deploys:

1. Go to **Components** > your web service > **Settings**
2. Under **Volumes**, click **Add Volume**
3. Set mount path to `/app/.db`
4. Update `DATABASE_URL` to `/app/.db/stark.db`

### 6. Deploy

Click **Create Resources** to deploy. The build takes a few minutes.

Your app will be available at: `https://your-app-name.ondigitalocean.app`

## App Spec (Alternative)

You can also deploy using a `.do/app.yaml` spec file:

```yaml
name: starkbot
services:
  - name: web
    dockerfile_path: Dockerfile
    github:
      repo: yourusername/starkbot
      branch: main
      deploy_on_push: true
    http_port: 8080
    health_check:
      http_path: /health
    instance_size_slug: basic-xxs
    instance_count: 1
    envs:
      - key: SECRET_KEY
        scope: RUN_TIME
        type: SECRET
      - key: RUST_LOG
        scope: RUN_TIME
        value: info
      - key: DATABASE_URL
        scope: RUN_TIME
        value: /app/.db/stark.db
```

Deploy with:
```bash
doctl apps create --spec .do/app.yaml
```

## Project Structure

```
starkbot/
├── Cargo.toml                 # Workspace manifest
├── Dockerfile                 # Multi-stage build
├── stark-backend/             # Actix web server
│   └── src/
│       ├── main.rs            # Server entry point
│       ├── config.rs          # Environment config
│       ├── db/sqlite.rs       # SQLite + sessions
│       ├── controllers/       # API endpoints
│       └── middleware/        # Auth middleware
└── stark-frontend/            # Static frontend
    ├── index.html             # Login page
    ├── dashboard.html         # Protected dashboard
    ├── css/styles.css
    └── js/
```
