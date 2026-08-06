# What this layer publishes. `api` reads the pool id and the issuer; the SPA
# build reads the client id and the hosted-UI domain. DR-0005.

resource "aws_ssm_parameter" "user_pool_id" {
  name  = "/${var.project}/identity/user_pool_id"
  type  = "String"
  value = aws_cognito_user_pool.this.id
}

# `endpoint` is the issuer without its scheme, which is not what a JWT `iss`
# claim looks like — the API's authorizer compares against the full URL.
resource "aws_ssm_parameter" "user_pool_issuer" {
  name  = "/${var.project}/identity/user_pool_issuer"
  type  = "String"
  value = "https://${aws_cognito_user_pool.this.endpoint}"
}

resource "aws_ssm_parameter" "app_client_id" {
  name  = "/${var.project}/identity/app_client_id"
  type  = "String"
  value = aws_cognito_user_pool_client.spa.id
}

resource "aws_ssm_parameter" "hosted_ui_domain" {
  name  = "/${var.project}/identity/hosted_ui_domain"
  type  = "String"
  value = local.hosted_ui_domain
}
