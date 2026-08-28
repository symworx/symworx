# Notes — backend cloud (AWS / Azure)

Not an SDK implementation plan. Records how this crate should talk to
cloud *when* a feature-gated client is added.

## Why no SDK on this branch

`aws-sdk-s3` and `azure_storage_blobs` are heavy (compile time, native
creds stacks). Workspace rule: new heavy deps stay feature-gated and
justified. Local disk plus a shared `ObjectStore` trait is enough to
wire ECS / App Service *config* now.

## Intended mapping

| Concern | Local | AWS | Azure |
|---------|-------|-----|-------|
| Objects | `LocalFsStore` under `SYMWORX_OBJECT_ROOT` | S3 bucket | Blob container |
| Prefix | `SYMWORX_OBJECT_PREFIX` | same key prefix | same |
| Region | n/a | `AWS_REGION` | `AZURE_LOCATION` |
| Dev endpoint | filesystem | LocalStack / MinIO | Azurite |
| Identity | none | task role / IRSA | managed identity |
| Secrets | env | Secrets Manager (later) | Key Vault (later) |
| Health | `HealthReport` | ALB / ECS health | App Service / Container Apps probe |

## What belongs here vs elsewhere

- **Here:** process table, health, opaque job artifacts, cloud *config*.
- **`symworx-io`:** on-disk signal formats (CSV, Parquet, IBI, FIT).
- **Not here:** analysis (RQA, TE, LMM), TUI, device streaming.

Research objects should stay de-identified. Prefix by study / site, not
by personal name. No PHI in keys.

## Later features (do not add until a consumer exists)

- `aws`: S3 `put`/`get`/`list` behind `ObjectStore`, default credential chain.
- `azure`: Blob client, default Azure credential.
- HTTP listener (axum) mounting `/healthz` and `/readyz` from `HealthReport`.
- Queue: SQS vs Storage Queues — only after a real job exists.

Pick one production cloud per deploy. Keep `SYMWORX_CLOUD` as the switch
so the same binary can run on a workstation (`local`) or a lab VM (`aws`
or `azure`) without code changes.
