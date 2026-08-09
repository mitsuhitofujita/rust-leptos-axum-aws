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

variable "point_in_time_recovery" {
  description = <<-EOT
    Continuous backups, restorable to any second in the last 35 days. On, because
    the failure this layer exists to survive is application-level — a bad write or
    a wrong delete — which neither prevent_destroy nor deletion protection sees.
  EOT
  type        = bool
  default     = true
}
