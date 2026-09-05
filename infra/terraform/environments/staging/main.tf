# Staging wiring for the placeholder app and data modules.
# No cloud provider is configured here on purpose.

terraform {
  required_version = ">= 1.5.0"
}

module "data" {
  source      = "../../modules/data"
  environment = "staging"
}

module "app" {
  source      = "../../modules/app"
  environment = "staging"
}

output "api_service_name" {
  value = module.app.api_service_name
}

output "data_hosts" {
  value = {
    postgres   = module.data.postgres_host
    redis      = module.data.redis_host
    opensearch = module.data.opensearch_host
    bucket     = module.data.object_storage_bucket
  }
}
