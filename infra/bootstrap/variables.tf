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
