# Configuration Guide

## Environment Variables

The application reads configuration from environment variables:

- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection for caching
- `JWT_SECRET` - Secret key for JWT token signing
- `LOG_LEVEL` - Logging verbosity (debug, info, warn, error)

## Config File

Additionally, a `config.yaml` file can override defaults:

```yaml
server:
  port: 8080
  host: 0.0.0.0
  workers: 4

database:
  pool_size: 10
  timeout_ms: 5000

auth:
  token_expiry: 86400
  max_failed_attempts: 5
```

## Initialization

The config module initializes in this order:
1. Load defaults from compiled-in values
2. Override with config.yaml if present
3. Override with environment variables
4. Validate all required values are set
