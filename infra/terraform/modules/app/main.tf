# Placeholder application module.
# This is not a production cloud. Images must stay free of secrets;
# inject connection strings and provider credentials from a secret
# manager or the process environment at deploy time.

variable "environment" {
  description = "Logical environment name such as staging."
  type        = string
}

variable "api_image" {
  description = "Container image for the API. Do not bake credentials into this image."
  type        = string
  default     = "market-bot-api:local"
}

variable "worker_image" {
  description = "Container image for the worker. Do not bake credentials into this image."
  type        = string
  default     = "market-bot-worker:local"
}

variable "web_image" {
  description = "Container image for the web frontend. Do not bake credentials into this image."
  type        = string
  default     = "market-bot-web:local"
}

output "api_service_name" {
  value = "market-bot-api-${var.environment}"
}

output "worker_service_name" {
  value = "market-bot-worker-${var.environment}"
}

output "web_service_name" {
  value = "market-bot-web-${var.environment}"
}

output "runtime_secret_names" {
  description = "Names of values the runtime must receive from a secret store."
  value = [
    "DATABASE_URL",
    "REDIS_URL",
    "S3_ENDPOINT",
    "S3_ACCESS_KEY",
    "S3_SECRET_KEY",
    "PAYMENT_PROVIDER",
  ]
}
