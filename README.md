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
UPLOAD_MAX_BYTES=5242880
```

Start Postgres for development:

```sh
docker compose -f docker-compose.dev-db.yml up -d
cargo run
```

The app runs migrations automatically and bootstraps the first admin account if no admin exists.

## Docker

Run the full app stack:

```sh
docker compose up --build
```

Open `http://localhost:8080`. The default Docker admin is `admin` / `change-me-now`; generate a fresh `ROCKET_SECRET_KEY` and change `ADMIN_PASSWORD` before using it anywhere public.

## Import existing Markdown articles

After the database is running, import the current `articles/*.md` files once:

```sh
cargo run --bin import_articles
```

The importer publishes missing articles by slug and does not overwrite existing DB articles.
