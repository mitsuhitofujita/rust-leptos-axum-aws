# What this layer publishes. The SPA build reads the endpoint; the deploy reads
# the function name. No layer reads either — api is the top of the stack.
# DR-0005.

resource "aws_ssm_parameter" "api_endpoint" {
  name  = "/${var.project}/api/api_endpoint"
  type  = "String"
  value = aws_apigatewayv2_stage.default.invoke_url
}

resource "aws_ssm_parameter" "lambda_function_name" {
  name  = "/${var.project}/api/lambda_function_name"
  type  = "String"
  value = aws_lambda_function.api.function_name
}

resource "aws_ssm_parameter" "ecr_repository_url" {
  name  = "/${var.project}/api/ecr_repository_url"
  type  = "String"
  value = aws_ecr_repository.api.repository_url
}
