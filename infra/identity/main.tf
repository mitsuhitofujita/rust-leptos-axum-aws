# The one value this layer takes from below it. Read through SSM rather than
# through delivery's state, so the contract between the layers is two strings
# rather than a whole state file — DR-0005.
data "aws_ssm_parameter" "cloudfront_domain" {
  name = "/${var.project}/delivery/cloudfront_domain"
}

locals {
  cloudfront_url   = "https://${nonsensitive(data.aws_ssm_parameter.cloudfront_domain.value)}"
  hosted_ui_domain = "${aws_cognito_user_pool_domain.this.domain}.auth.${var.region}.amazoncognito.com"
}

# ---------------------------------------------------------------------------
# The user pool
#
# The most expensive thing in the stack to destroy: nothing recreates the
# identities in it. Hence both guards — prevent_destroy stops Terraform, and
# deletion_protection stops the console and the CLI too. DR-0005.
# ---------------------------------------------------------------------------

resource "aws_cognito_user_pool" "this" {
  name                     = var.project
  deletion_protection      = "ACTIVE"
  auto_verified_attributes = ["email"]
  mfa_configuration        = "OFF"

  account_recovery_setting {
    recovery_mechanism {
      name     = "verified_email"
      priority = 1
    }
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_cognito_identity_provider" "google" {
  user_pool_id  = aws_cognito_user_pool.this.id
  provider_name = "Google"
  provider_type = "Google"

  provider_details = {
    client_id        = var.google_client_id
    client_secret    = var.google_client_secret
    authorize_scopes = "openid email profile"
  }

  # Google's `sub` becomes the pool's username, so a user who changes their
  # Google email address stays the same user.
  attribute_mapping = {
    email    = "email"
    username = "sub"
  }
}

resource "aws_cognito_user_pool_domain" "this" {
  domain       = var.hosted_ui_domain_prefix
  user_pool_id = aws_cognito_user_pool.this.id
}

# ---------------------------------------------------------------------------
# The app client
#
# Public and secretless: a secret cannot be kept in a WASM bundle the browser
# downloads, so the authorization code flow is secured by PKCE instead. Cognito
# requires PKCE of any public client, so there is no argument to set for it.
# ---------------------------------------------------------------------------

resource "aws_cognito_user_pool_client" "spa" {
  name         = "${var.project}-spa"
  user_pool_id = aws_cognito_user_pool.this.id

  generate_secret = false

  allowed_oauth_flows_user_pool_client = true
  allowed_oauth_flows                  = ["code"]
  allowed_oauth_scopes                 = ["openid", "email", "profile"]
  supported_identity_providers         = [aws_cognito_identity_provider.google.provider_name]

  # Production and `trunk serve` share one client, which is why both origins
  # appear here. Trunk.toml fixes the local port at 8080.
  callback_urls = ["${local.cloudfront_url}/", "${var.local_dev_url}/"]
  logout_urls   = ["${local.cloudfront_url}/", "${var.local_dev_url}/"]

  prevent_user_existence_errors = "ENABLED"
}
