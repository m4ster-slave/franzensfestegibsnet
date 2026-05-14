## Franzensfeste gibs net 
Franzensfeste gibs (dot) net (eng: Franzensfeste doesnt exist), is a joke website based about a local town me and my friends make fun of for being ominous and weird in general 

## Run locally

Create a `.env` with:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/franzensfestegibsnet
ROCKET_SECRET_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
ADMIN_USERNAME=admin
ADMIN_EMAIL=admin@example.local
ADMIN_PASSWORD=change-me-now
UPLOAD_DIR=./uploads
UPLOAD_MAX_BYTES=12582912
SECURE_COOKIES=false
```

Start Postgres for development:

```sh
docker compose -f docker-compose.dev-db.yml up -d
cargo run
```

The app runs migrations automatically and bootstraps the first admin account if no admin exists.

## Docker

Run the full app stack from the published GitHub Container Registry image:

```sh
docker compose pull
docker compose up -d
```

Open `http://localhost:8080`. The default Docker admin is `admin` / `change-me-now`; generate a fresh `ROCKET_SECRET_KEY` and change `ADMIN_PASSWORD` before using it anywhere public.

For a VPS, keep your production values in `.env` next to `docker-compose.yml`:

```env
ROCKET_SECRET_KEY=replace-with-a-long-random-secret
ADMIN_PASSWORD=replace-with-a-real-password
POSTGRES_PASSWORD=replace-with-a-real-db-password
APP_PORT=8080
SECURE_COOKIES=true
```

Do not set `DATABASE_URL` for the bundled Postgres container; Compose wires the app to the `db` service automatically.

The image is published on every push to `main` as:

```text
ghcr.io/m4ster-slave/franzensfestegibsnet:latest
```

## Import existing Markdown articles

After the database is running, import the current `articles/*.md` files once:

```sh
cargo run --bin import_articles
```

The importer publishes missing articles by slug and does not overwrite existing DB articles.
