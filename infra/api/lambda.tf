# The bytes uploaded at create time and never again — see placeholder/bootstrap
# and the ignore_changes below.
data "archive_file" "placeholder" {
  type             = "zip"
  source_file      = "${path.module}/placeholder/bootstrap"
  output_file_mode = "0755"
  output_path      = "${path.module}/.terraform/placeholder.zip"
}

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

  # crates/server is an ordinary axum binary, unmodified for Lambda. The custom
  # runtime convention names the packaged executable `bootstrap`; the Lambda Web
  # Adapter layer separately provides its own /opt/bootstrap. The two names are
  # unrelated.
  runtime = "provided.al2023"
  handler = "bootstrap"
  layers  = [var.lambda_web_adapter_layer_arn]

  filename         = data.archive_file.placeholder.output_path
  source_code_hash = data.archive_file.placeholder.output_base64sha256

  environment {
    variables = {
      # Makes the adapter the entry point and the packaged binary the process it
      # proxies to.
      AWS_LAMBDA_EXEC_WRAPPER = "/opt/bootstrap"
      # crates/server binds 127.0.0.1:3000 as a constant; the adapter defaults to
      # 8080. Nothing checks that these two agree — docs/design/deployment.md is
      # what keeps them in step.
      AWS_LWA_PORT = "3000"
      # The endpoint crates/server already serves.
      AWS_LWA_READINESS_CHECK_PATH = "/health"
      # The table the service reads and writes. Passed rather than derived, so
      # the name lives in one place — the data layer — and travels through SSM
      # like every other cross-layer value. crates/server does not read it yet.
      TABLE_NAME = local.table_name
    }
  }

  # Artefacts deploy on their own cadence, by `aws lambda update-function-code`.
  # Without this, every apply would revert the function to the placeholder and
  # undo the last deploy.
  lifecycle {
    ignore_changes = [filename, source_code_hash]
  }

  depends_on = [
    aws_iam_role_policy_attachment.lambda_basic,
    aws_iam_role_policy.lambda_table,
    aws_cloudwatch_log_group.lambda,
  ]
}

resource "aws_lambda_permission" "api_gateway" {
  statement_id  = "AllowExecutionFromApiGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.api.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.this.execution_arn}/*/*"
}
