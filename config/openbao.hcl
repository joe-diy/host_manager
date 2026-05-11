# OpenBao (Vault-compatible) configuration for Host Manager
#
# Dev mode (docker compose): OpenBao is started with `server -dev` and this
# file is not used. It is provided here as a reference for production deployments.
#
# Production: mount this file into the OpenBao container and start with:
#   bao server -config=/vault/config/openbao.hcl

ui = true

storage "file" {
  path = "/vault/data"
}

listener "tcp" {
  address       = "0.0.0.0:8200"
  tls_cert_file = "/vault/tls/server.crt"
  tls_key_file  = "/vault/tls/server.key"
  # Require TLS 1.2+ (TLS 1.3 preferred)
  tls_min_version = "tls12"
}

# Default lease TTLs
default_lease_ttl = "168h"   # 7 days
max_lease_ttl     = "720h"   # 30 days

# Enable KV v2 secret engine at secret/
# (provisioned by the init script; not set in HCL)

# Telemetry — optional; uncomment for Prometheus scraping
# telemetry {
#   prometheus_retention_time = "30s"
#   disable_hostname = true
# }

# Audit log — strongly recommended in production
# audit {
#   path = "file/"
#   options = {
#     file_path = "/vault/logs/audit.log"
#   }
# }
