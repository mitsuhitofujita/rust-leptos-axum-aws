# Created here rather than left to Lambda, so that retention is set and the group
# is destroyed with the layer. Lambda would otherwise create it on first
# invocation with retention set to never expire.
resource "aws_cloudwatch_log_group" "lambda" {
  name              = "/aws/lambda/${local.name}"
  retention_in_days = var.log_retention_days
}

resource "aws_lambda_function" "api" {
  function_name = local.name
  role          = aws_iam_role.lambda.arn
  architectures = [var.lambda_architecture]
  memory_size   = var.lambda_memory_size
  timeout       = var.lambda_timeout

  # crates/server is an ordinary axum binary, unmodified for Lambda — only how
  # it is packaged changes here. infra/api/Dockerfile builds it into an image
  # carrying the Lambda Web Adapter as an extension rather than a layer;
  # AWS_LAMBDA_EXEC_WRAPPER, the setting that made the adapter layer's
  # /opt/bootstrap the entry point, has no equivalent below because the image's
  # own ENTRYPOINT is the service directly.
  package_type = "Image"

  # Resolves to whatever `just deploy-api` most recently pushed under this tag.
  # A later push moves the tag; ignore_changes below is what keeps a later
  # apply from reading that as drift and reverting the function to whatever
  # this expression evaluates to at plan time.
  image_uri = "${aws_ecr_repository.api.repository_url}:latest"

  environment {
    variables = {
      # crates/server binds 127.0.0.1:3000 as a constant; the adapter defaults to
      # 8080. Nothing checks that these two agree — docs/design/deployment.md is
      # what keeps them in step.
      AWS_LWA_PORT = "3000"
      # The endpoint crates/server already serves.
      AWS_LWA_READINESS_CHECK_PATH = "/health"
      # The table the service reads and writes. Passed rather than derived, so
      # the name lives in one place — the data layer — and travels through SSM
      # like every other cross-layer value.
      TABLE_NAME = local.table_name

      # What aws_apigatewayv2_authorizer.cognito's jwt_configuration used to
      # hold; the service verifies against these directly now (DR-0028).
      # Passed rather than derived, so the value lives in one place — the
      # identity layer — and travels through SSM like every other cross-layer
      # value. Both set is what selects identity::Auth::Cognito over the
      # header-trusting Auth::Mock at startup.
      COGNITO_ISSUER   = local.user_pool_issuer
      COGNITO_AUDIENCE = local.app_client_id
    }
  }

  # Artefacts deploy on their own cadence, by `aws lambda update-function-code`.
  # Without this, every apply would revert the function to whatever `:latest`
  # resolved to at the previous apply and undo the last deploy.
  lifecycle {
    ignore_changes = [image_uri]
  }

  depends_on = [
    aws_iam_role_policy_attachment.lambda_basic,
    aws_iam_role_policy.lambda_table,
    aws_cloudwatch_log_group.lambda,
    aws_ecr_repository_policy.api,
  ]
}

resource "aws_lambda_permission" "api_gateway" {
  statement_id  = "AllowExecutionFromApiGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.api.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.this.execution_arn}/*/*"
}
