variable "project" {
  description = "Resource name prefix and the root of every SSM parameter path."
  type        = string
  default     = "rust-leptos-axum-aws"
}

variable "region" {
  description = "The single region everything lives in."
  type        = string
  default     = "ap-northeast-1"
}

variable "lambda_architecture" {
  description = <<-EOT
    x86_64, matching the native architecture infra/api/Dockerfile builds on.
    Moving to arm64 would mean building the image on an arm64 host or
    cross-building it, and the Lambda Web Adapter image tag in that Dockerfile
    would need its arm64 variant.
  EOT
  type        = string
  default     = "x86_64"
}

variable "lambda_memory_size" {
  description = "Megabytes. CPU is allocated in proportion to this."
  type        = number
  default     = 512
}

variable "lambda_timeout" {
  description = "Seconds. The HTTP API's own limit is 30, so nothing above that is reachable."
  type        = number
  default     = 30
}

variable "log_retention_days" {
  description = "Retention on both log groups."
  type        = number
  default     = 14
}
