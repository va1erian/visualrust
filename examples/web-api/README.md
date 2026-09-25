# web-api — JSON REST API backed by SQLite

A VisualRust **web** sample. `src/main.dyon` registers named handlers with the
message-based web runtime (`vr-web`, #86) and reads/writes SQLite through the
`vr-db` natives. Each HTTP request is turned into a call to the Dyon function
named in `vrproj.toml`, and the handler's returned `{status, body,
content_type}` object becomes the response.

## Routes

| Method | Path         | Handler       | Description                         |
| ------ | ------------ | ------------- | ----------------------------------- |
| GET    | `/`          | `index`       | Service banner (JSON)               |
| POST   | `/notes`     | `create_note` | Insert a note; body is the text     |
| GET    | `/notes`     | `list_notes`  | List every note as a JSON array     |
| GET    | `/notes/:id` | `show_note`   | Read one note by id (JSON or `404`) |

The database file defaults to `web-api.sqlite` in the working directory; the
`CREATE TABLE IF NOT EXISTS` runs once at startup.

## Running

The runtime binds `127.0.0.1:0`, so `server_start` would normally report the
ephemeral port. From the IDE or the packaged runtime, run the project as a web
app; for the checked-in sample the automated smoke test in `vr-web` is the
supported way to exercise it:

```
cargo test -p vr-web --features sqlite --test web_api_sample
```

The test reads this project's `vrproj.toml` and `src/main.dyon`, points the
database at a temp file, starts the server on an ephemeral port and drives it
over a real `TcpStream` — create a row, then GET it back.

## curl examples

Against a running instance (replace `PORT` with the bound port):

```sh
# Service banner
curl -s http://127.0.0.1:PORT/

# Create a note
curl -s -X POST --data 'first note' http://127.0.0.1:PORT/notes
# => {"id": 1, "body": "first note"}

# Read it back by id
curl -s http://127.0.0.1:PORT/notes/1
# => {"id": 1, "body": "first note"}

# List every note
curl -s http://127.0.0.1:PORT/notes
# => [{"id": 1, "body": "first note"}]

# Missing rows are a JSON 404
curl -s -i http://127.0.0.1:PORT/notes/999
```
