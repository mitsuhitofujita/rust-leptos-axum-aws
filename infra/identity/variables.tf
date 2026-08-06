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

variable "hosted_ui_domain_prefix" {
  description = <<-EOT
    Prefix of the Cognito hosted-UI domain. Globally unique across all of AWS,
    so a first apply can fail on it even though nothing here is wrong.

    Deliberately not derived from var.project: Cognito rejects a prefix
    containing "aws", "amazon" or "cognito", and the project name contains one
    of them. This is the only name in the stack that cannot echo the project.
  EOT
  type        = string
  default     = "rust-leptos-axum-auth"
}

variable "local_dev_url" {
  description = <<-EOT
    The trunk dev server, listed alongside the CloudFront domain in the app
    client's callback and logout URLs. One client serves production and local
    development alike — this is not a second environment.
  EOT
  type        = string
  default     = "http://localhost:8080"
}

# Real values never enter this repository. Pass them with a gitignored
# *.auto.tfvars or with TF_VAR_google_client_id / TF_VAR_google_client_secret.

variable "google_client_id" {
  description = "OAuth client id from the Google Cloud console."
  type        = string
}

variable "google_client_secret" {
  description = "OAuth client secret from the Google Cloud console."
  type        = string
  sensitive   = true
}
