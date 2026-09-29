# Model list API (format version 1)

KuttyLM's "Picked for you" list comes from a small JSON API. You can run your own.

- Set the URL in `src-tauri/src/config.rs` (`MODELS_API_URL`). While it is empty, the app uses the
  built-in sample list [`src-tauri/resources/models.mock.json`](../src-tauri/resources/models.mock.json),
  which is a complete, working example of this format.
- For testing, the environment variable `KUTTYLM_MODELS_API_URL` overrides the URL.
- HTTPS is required. Plain HTTP is allowed only for `localhost` / `127.0.0.1` (testing).
- The API is public: the app sends no key. It waits 10 seconds at most.

## Response

`GET <MODELS_API_URL>` must answer `200` with JSON like this:

```json
{
  "version": 1,
  "updated_at": "2026-09-26T10:00:00Z",
  "models": [
    {
      "id": "qwen2.5-0.5b-instruct-q4km",
      "name": "Qwen2.5 0.5B Instruct",
      "description": "Very small and fast. Good for testing and simple questions.",
      "hf_repo": "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
      "revision": "9217f5db79a29953eb74d5343926648285ec7e67",
      "files": [
        {
          "name": "qwen2.5-0.5b-instruct-q4_k_m.gguf",
          "size_bytes": 491400032,
          "sha256": "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db"
        }
      ],
      "mmproj": null,
      "quantization": "Q4_K_M",
      "params_b": 0.5,
      "context_length": 32768,
      "license": "apache-2.0",
      "tags": ["chat"],
      "trending": true
    }
  ]
}
```

## Fields

| Field | Required | Meaning |
|---|---|---|
| `version` | yes | Format version. The app reads `1`. A higher number shows "Please update the app". |
| `models` | yes | The list. |
| `id` | yes | Unique id. Also the folder name on disk, so only letters, digits, `.`, `_`, `-`. |
| `name` | yes | Name shown to the user. |
| `hf_repo` | yes | Hugging Face repository, `owner/name`. |
| `revision` | no | Branch or commit id (default `main`). A commit id is safer: the files cannot change. |
| `files` | yes | The GGUF file(s). Several for split models (`…-00001-of-00003.gguf`), in order. |
| `files[].name` | yes | File name in the repository root. Must end in `.gguf`; no `/`. |
| `files[].size_bytes` | yes | Exact size. Used for the fit check, disk check and download check. |
| `files[].sha256` | no | If given, the download is checked against it. |
| `mmproj` | no | Extra file for vision models, same shape as a `files` item. |
| `description`, `quantization`, `params_b`, `context_length`, `license`, `tags` | no | Shown in the app. |
| `trending` | no | `true` shows a "Popular" tag and puts the model first. |

Files are downloaded from `https://huggingface.co/{hf_repo}/resolve/{revision}/{file name}`.

## How the app treats the list

- Unknown fields are ignored; missing optional fields are fine.
- A broken or unsafe entry is skipped; the rest of the list still shows. Duplicate ids are skipped.
- Model size = all `files` + `mmproj`. The "Fits well / May be slow / Will not fit" label uses it.
- The last good answer is cached. Offline, the app shows it with "Offline – list from <date>".
  If the server sends an `ETag`, the app asks with `If-None-Match` and accepts `304 Not Modified`.
- Do not list gated models (models that need a Hugging Face login): the app cannot download them.
