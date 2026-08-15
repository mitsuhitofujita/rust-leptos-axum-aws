output "api_endpoint" {
  description = "The base URL the SPA is built against."
  value       = aws_apigatewayv2_stage.default.invoke_url
}

output "lambda_function_name" {
  description = "Target of `aws lambda update-function-code` after a build."
  value       = aws_lambda_function.api.function_name
}

output "ecr_repository_url" {
  description = "Where `just deploy-api` builds and pushes the image."
  value       = aws_ecr_repository.api.repository_url
}
