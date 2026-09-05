# Placeholder data-plane module for PostgreSQL, Redis, OpenSearch,
# object storage, and the transactional outbox store.
# This is not a production cloud and does not create real resources
# or embed credentials.

variable "environment" {
  description = "Logical environment name such as staging."
  type        = string
}

output "postgres_host" {
  value = "market-bot-postgres-${var.environment}"
}

output "redis_host" {
  value = "market-bot-redis-${var.environment}"
}

output "opensearch_host" {
  value = "market-bot-opensearch-${var.environment}"
}

output "object_storage_bucket" {
  value = "market-bot-${var.environment}"
}

output "outbox_table" {
  value = "outbox_events"
}

output "connection_placeholders" {
  description = "Non-secret connection shapes. Real credentials come from a secret manager."
  value = {
    database_url = "postgres://USER:SECRET@market-bot-postgres-${var.environment}:5432/marketbot"
    redis_url    = "redis://market-bot-redis-${var.environment}:6379"
    s3_endpoint  = "https://object-storage.example.invalid"
    opensearch_url = "https://opensearch.example.invalid"
  }
}
