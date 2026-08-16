output "user_pool_id" {
  description = "The Cognito user pool holding every identity."
  value       = aws_cognito_user_pool.this.id
}

output "user_pool_issuer" {
  description = "The `iss` claim crates/server's own Cognito verification validates against."
  value       = "https://${aws_cognito_user_pool.this.endpoint}"
}

output "app_client_id" {
  description = "The public PKCE client id, compiled into the SPA bundle."
  value       = aws_cognito_user_pool_client.spa.id
}

output "hosted_ui_domain" {
  description = "Where the SPA sends the browser to sign in."
  value       = local.hosted_ui_domain
}
