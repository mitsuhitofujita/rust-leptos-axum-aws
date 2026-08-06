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

variable "lambda_web_adapter_layer_arn" {
  description = <<-EOT
    The AWS Lambda Web Adapter layer, published by AWS into its own account and
    therefore both region- and architecture-specific. Version 28 is adapter
    1.0.1, x86_64 only. The layer is named LambdaAdapterLayerX86, not
    LambdaAdapterLayerX86_64.
  EOT
  type        = string
  default     = "arn:aws:lambda:ap-northeast-1:753240598075:layer:LambdaAdapterLayerX86:28"
}

variable "lambda_architecture" {
  description = <<-EOT
    x86_64, matching the devcontainer's native Rust target. Moving to arm64
    would mean cross-compiling crates/server and switching to the adapter's
    LambdaAdapterLayerArm64 layer.
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
